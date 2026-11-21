//! Low-level keyboard and mouse hooks on their own thread with a message loop.
//! The hook procedures only forward events to a channel, so they stay fast.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use windows::core::w;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, MAPVK_VK_TO_VSC_EX};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, RegisterClassW,
    SetWindowsHookExW, TranslateMessage, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WH_MOUSE_LL, WM_DISPLAYCHANGE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP,
    WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    Down(u32),
    Up(u32),
    MouseDelta(i32, i32),
    /// Windows changed the screen layout, a resolution or a graphics card.
    DisplayChanged,
}

static SENDER: OnceLock<Mutex<Sender<HookEvent>>> = OnceLock::new();
/// Set while the look wheel is open, so mouse moves are forwarded.
pub static TRACK_MOUSE: AtomicBool = AtomicBool::new(false);
static PRESSED: [AtomicU64; 4] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

fn send(e: HookEvent) {
    if let Some(s) = SENDER.get() {
        if let Ok(s) = s.lock() {
            let _ = s.send(e);
        }
    }
}

/// Returns true the first time a key goes down (ignores auto-repeat).
fn mark(vk: u32, down: bool) -> bool {
    let vk = (vk & 0xFF) as usize;
    let word = &PRESSED[vk / 64];
    let bit = 1u64 << (vk % 64);
    if down {
        word.fetch_or(bit, Ordering::Relaxed) & bit == 0
    } else {
        word.fetch_and(!bit, Ordering::Relaxed) & bit != 0
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let k = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if k.flags.0 & LLKHF_INJECTED.0 == 0 {
            let msg = wparam.0 as u32;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                if mark(k.vkCode, true) {
                    send(HookEvent::Down(k.vkCode));
                }
            } else if (msg == WM_KEYUP || msg == WM_SYSKEYUP) && mark(k.vkCode, false) {
                send(HookEvent::Up(k.vkCode));
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let m = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let msg = wparam.0 as u32;
        let x_button = || if (m.mouseData >> 16) & 0xFFFF == 2 { 6 } else { 5 };
        match msg {
            WM_LBUTTONDOWN => send(HookEvent::Down(1)),
            WM_LBUTTONUP => send(HookEvent::Up(1)),
            WM_RBUTTONDOWN => send(HookEvent::Down(2)),
            WM_RBUTTONUP => send(HookEvent::Up(2)),
            WM_MBUTTONDOWN => send(HookEvent::Down(4)),
            WM_MBUTTONUP => send(HookEvent::Up(4)),
            WM_XBUTTONDOWN => send(HookEvent::Down(x_button())),
            WM_XBUTTONUP => send(HookEvent::Up(x_button())),
            WM_MOUSEMOVE if TRACK_MOUSE.load(Ordering::Relaxed) => {
                // The cursor has not moved yet: the difference is this move, even
                // when a game keeps re-centring the cursor.
                let mut cur = POINT::default();
                if GetCursorPos(&mut cur).is_ok() {
                    send(HookEvent::MouseDelta(m.pt.x - cur.x, m.pt.y - cur.y));
                }
            }
            _ => {}
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn listener_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_DISPLAYCHANGE {
        send(HookEvent::DisplayChanged);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// A window that is never shown: it only receives the WM_DISPLAYCHANGE broadcast
/// (message-only windows do not get broadcasts).
unsafe fn create_display_listener() {
    let class = w!("LunaDisplayListener");
    let instance = GetModuleHandleW(None).map(|h| HINSTANCE(h.0)).unwrap_or_default();
    let wc = WNDCLASSW { lpfnWndProc: Some(listener_proc), hInstance: instance, lpszClassName: class, ..Default::default() };
    RegisterClassW(&wc);
    let _ = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        class,
        w!("Luna display listener"),
        WS_POPUP,
        0,
        0,
        0,
        0,
        None,
        None,
        Some(instance),
        None,
    );
}

/// Starts the hook thread. Events arrive on `tx`.
pub fn start(tx: Sender<HookEvent>) {
    let _ = SENDER.set(Mutex::new(tx));
    std::thread::Builder::new()
        .name("luna-input-hooks".into())
        .spawn(|| unsafe {
            create_display_listener();
            // Even without input hooks, the loop below still serves the display listener.
            let _ = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0);
            let _ = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("hook thread");
}

pub fn is_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

/// A readable name for a virtual-key code.
pub fn key_label(vk: u32) -> String {
    let fixed = match vk {
        1 => Some("Mouse 1"),
        2 => Some("Mouse 2"),
        4 => Some("Mouse 3"),
        5 => Some("Mouse 4"),
        6 => Some("Mouse 5"),
        0x08 => Some("Backspace"),
        0x09 => Some("Tab"),
        0x0D => Some("Enter"),
        0x10 | 0xA0 => Some("Shift"),
        0xA1 => Some("Right Shift"),
        0x11 | 0xA2 => Some("Ctrl"),
        0xA3 => Some("Right Ctrl"),
        0x12 | 0xA4 => Some("Alt"),
        0xA5 => Some("Right Alt"),
        0x13 => Some("Pause"),
        0x14 => Some("Caps Lock"),
        0x1B => Some("Esc"),
        0x20 => Some("Space"),
        0x21 => Some("Page Up"),
        0x22 => Some("Page Down"),
        0x23 => Some("End"),
        0x24 => Some("Home"),
        0x25 => Some("Left"),
        0x26 => Some("Up"),
        0x27 => Some("Right"),
        0x28 => Some("Down"),
        0x2C => Some("Print Screen"),
        0x2D => Some("Insert"),
        0x2E => Some("Delete"),
        0x5B => Some("Left Win"),
        0x5C => Some("Right Win"),
        0x90 => Some("Num Lock"),
        0x91 => Some("Scroll Lock"),
        _ => None,
    };
    if let Some(f) = fixed {
        return f.to_string();
    }
    if (0x70..=0x87).contains(&vk) {
        return format!("F{}", vk - 0x6F);
    }
    if (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) {
        return char::from_u32(vk).map(|c| c.to_string()).unwrap_or_default();
    }
    if (0x60..=0x69).contains(&vk) {
        return format!("Num {}", vk - 0x60);
    }
    unsafe {
        let sc = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC_EX);
        if sc != 0 {
            let mut buf = [0u16; 64];
            let lparam = ((sc & 0xFF) << 16) | if sc & 0xE000 != 0 { 1 << 24 } else { 0 };
            let n = GetKeyNameTextW(lparam as i32, &mut buf);
            if n > 0 {
                return String::from_utf16_lossy(&buf[..n as usize]);
            }
        }
    }
    format!("Key {vk}")
}
