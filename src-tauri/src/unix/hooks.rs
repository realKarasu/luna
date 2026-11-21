//! Keys and mouse buttons by polling the system about every 8 ms (device_query), turned into
//! the same Windows virtual-key codes the Windows hooks report, so binds work the same way.
//!
//! macOS asks for the Accessibility permission the first time; until it is given, polling waits.
//! On Linux this reads the X server, so under Wayland only X11 (XWayland) programs are seen.

pub use crate::input::{HookEvent, TRACK_MOUSE};
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

const POLL: Duration = Duration::from_millis(8);
const RETRY: Duration = Duration::from_secs(3);

#[allow(clippy::declare_interior_mutable_const)]
const UP: AtomicBool = AtomicBool::new(false);
static DOWN: [AtomicBool; 256] = [UP; 256];
static CURSOR_X: AtomicI32 = AtomicI32::new(0);
static CURSOR_Y: AtomicI32 = AtomicI32::new(0);

pub(super) fn cursor() -> (i32, i32) {
    (CURSOR_X.load(Ordering::Relaxed), CURSOR_Y.load(Ordering::Relaxed))
}

#[rustfmt::skip]
fn vk(k: &Keycode) -> Option<u32> {
    use Keycode::*;
    let v = match k {
        Key0 => 0x30, Key1 => 0x31, Key2 => 0x32, Key3 => 0x33, Key4 => 0x34,
        Key5 => 0x35, Key6 => 0x36, Key7 => 0x37, Key8 => 0x38, Key9 => 0x39,
        A => 0x41, B => 0x42, C => 0x43, D => 0x44, E => 0x45, F => 0x46, G => 0x47,
        H => 0x48, I => 0x49, J => 0x4A, K => 0x4B, L => 0x4C, M => 0x4D, N => 0x4E,
        O => 0x4F, P => 0x50, Q => 0x51, R => 0x52, S => 0x53, T => 0x54, U => 0x55,
        V => 0x56, W => 0x57, X => 0x58, Y => 0x59, Z => 0x5A,
        F1 => 0x70, F2 => 0x71, F3 => 0x72, F4 => 0x73, F5 => 0x74, F6 => 0x75, F7 => 0x76,
        F8 => 0x77, F9 => 0x78, F10 => 0x79, F11 => 0x7A, F12 => 0x7B, F13 => 0x7C,
        F14 => 0x7D, F15 => 0x7E, F16 => 0x7F, F17 => 0x80, F18 => 0x81, F19 => 0x82, F20 => 0x83,
        Escape => 0x1B, Space => 0x20,
        LControl => 0xA2, RControl => 0xA3, LShift => 0xA0, RShift => 0xA1,
        LAlt | LOption => 0xA4, RAlt | ROption => 0xA5,
        Command | LMeta => 0x5B, RCommand | RMeta => 0x5C,
        Enter | NumpadEnter => 0x0D,
        Up => 0x26, Down => 0x28, Left => 0x25, Right => 0x27,
        Backspace => 0x08, CapsLock => 0x14, Tab => 0x09,
        Home => 0x24, End => 0x23, PageUp => 0x21, PageDown => 0x22, Insert => 0x2D, Delete => 0x2E,
        Numpad0 => 0x60, Numpad1 => 0x61, Numpad2 => 0x62, Numpad3 => 0x63, Numpad4 => 0x64,
        Numpad5 => 0x65, Numpad6 => 0x66, Numpad7 => 0x67, Numpad8 => 0x68, Numpad9 => 0x69,
        NumpadSubtract => 0x6D, NumpadAdd => 0x6B, NumpadDivide => 0x6F, NumpadMultiply => 0x6A,
        NumpadDecimal => 0x6E, NumpadEquals => 0xBB,
        Grave => 0xC0, Minus => 0xBD, Equal => 0xBB, LeftBracket => 0xDB, RightBracket => 0xDD,
        BackSlash => 0xDC, Semicolon => 0xBA, Apostrophe => 0xDE, Comma => 0xBC, Dot => 0xBE,
        Slash => 0xBF,
        #[allow(unreachable_patterns)]
        _ => return None,
    };
    Some(v)
}

/// device_query's button index to the Windows code: 1 left, 2 right, 4 middle.
fn button_vk(i: usize) -> Option<u32> {
    if cfg!(target_os = "macos") {
        match i {
            1 => Some(1),
            2 => Some(2),
            3 => Some(4),
            _ => None,
        }
    } else {
        // X11: 1 left, 2 middle, 3 right; 4 and 5 are the scroll wheel.
        match i {
            1 => Some(1),
            2 => Some(4),
            3 => Some(2),
            _ => None,
        }
    }
}

#[cfg(target_os = "macos")]
fn open_state(first: bool) -> Option<DeviceState> {
    // The first try shows the system's permission prompt; later ones check quietly.
    if first || crate::mac::accessibility_granted() {
        DeviceState::checked_new()
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn open_state(_first: bool) -> Option<DeviceState> {
    DeviceState::checked_new()
}

pub fn start(tx: Sender<HookEvent>) {
    std::thread::Builder::new()
        .name("luna-input".into())
        .spawn(move || {
            let mut first = true;
            let state = loop {
                if let Some(s) = open_state(first) {
                    break s;
                }
                first = false;
                std::thread::sleep(RETRY);
            };
            let mut held: HashSet<u32> = HashSet::new();
            let mut last: Option<(i32, i32)> = None;
            loop {
                let mouse = state.get_mouse();
                let mut now: HashSet<u32> = state.get_keys().iter().filter_map(vk).collect();
                for (i, pressed) in mouse.button_pressed.iter().enumerate() {
                    if let (true, Some(v)) = (*pressed, button_vk(i)) {
                        now.insert(v);
                    }
                }
                let mut events = Vec::new();
                for &v in now.difference(&held) {
                    DOWN[v as usize & 0xFF].store(true, Ordering::Relaxed);
                    events.push(HookEvent::Down(v));
                }
                for &v in held.difference(&now) {
                    DOWN[v as usize & 0xFF].store(false, Ordering::Relaxed);
                    events.push(HookEvent::Up(v));
                }
                held = now;

                let (x, y) = mouse.coords;
                CURSOR_X.store(x, Ordering::Relaxed);
                CURSOR_Y.store(y, Ordering::Relaxed);
                if TRACK_MOUSE.load(Ordering::Relaxed) {
                    if let Some((lx, ly)) = last {
                        if (x, y) != (lx, ly) {
                            events.push(HookEvent::MouseDelta(x - lx, y - ly));
                        }
                    }
                }
                last = Some((x, y));

                for e in events {
                    if tx.send(e).is_err() {
                        return;
                    }
                }
                std::thread::sleep(POLL);
            }
        })
        .ok();
}

/// Whether a key is held. Shift, Ctrl and Alt count when either side is.
pub fn is_down(vk: i32) -> bool {
    let d = |v: usize| DOWN[v].load(Ordering::Relaxed);
    match vk {
        0x10 => d(0x10) || d(0xA0) || d(0xA1),
        0x11 => d(0x11) || d(0xA2) || d(0xA3),
        0x12 => d(0x12) || d(0xA4) || d(0xA5),
        v if (0..256).contains(&v) => d(v as usize),
        _ => false,
    }
}

pub fn key_label(vk: u32) -> String {
    crate::input::basic_label(vk).unwrap_or_else(|| format!("Key {vk}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys() {
        assert_eq!(vk(&Keycode::A), Some(0x41));
        assert_eq!(vk(&Keycode::F12), Some(0x7B));
        assert_eq!(vk(&Keycode::LShift), Some(0xA0));
        assert_eq!(button_vk(1), Some(1));
        assert_eq!(button_vk(0), None);
    }
}
