//! Borderless windowed: takes the title bar and frame off another program's window, and keeps
//! its picture where it was or sizes it to its screen. The frame and place it had are kept so it can go back.
//!
//! Changing a style waits for the game to answer, so nothing here runs under the engine lock.

use crate::fit::{self, Rect, WindowInfo};
use crate::screens::MonitorInfo;
use crate::settings::{Borderless, Fit};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Duration;
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, SetLastError, ERROR_ACCESS_DENIED, HWND, LPARAM, POINT, RECT, WIN32_ERROR};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{ClientToScreen, GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SetWindowLongPtrW, SetWindowPos,
    ShowWindowAsync, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, HWND_TOP, SWP_ASYNCWINDOWPOS, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOOWNERZORDER, SWP_NOZORDER, SW_RESTORE, WS_CAPTION, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE,
    WS_EX_TOOLWINDOW, WS_EX_WINDOWEDGE, WS_MAXIMIZE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
};

const GONE: &str = "That window is closed.";
const ADMIN: &str = "This game runs as administrator, so Windows does not let Luna change its window. Start Luna as administrator too.";

/// What a window looked like before Luna took its frame off.
struct Saved {
    style: isize,
    ex: isize,
    rect: Rect,
}

static SAVED: LazyLock<Mutex<HashMap<isize, Saved>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn saved() -> MutexGuard<'static, HashMap<isize, Saved>> {
    SAVED.lock().unwrap_or_else(|e| e.into_inner())
}

fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

fn alive(h: HWND) -> bool {
    unsafe { IsWindow(Some(h)).as_bool() }
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(h, Some(&mut pid));
    }
    pid
}

fn frame_bits() -> isize {
    (WS_CAPTION | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX).0 as isize
}

fn edge_bits() -> isize {
    (WS_EX_DLGMODALFRAME | WS_EX_CLIENTEDGE | WS_EX_STATICEDGE | WS_EX_WINDOWEDGE).0 as isize
}

fn framed(h: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(h, GWL_STYLE) };
    style & (WS_CAPTION | WS_THICKFRAME).0 as isize != 0
}

fn text(buf: &[u16], len: i32) -> String {
    String::from_utf16_lossy(&buf[..len.clamp(0, buf.len() as i32) as usize])
}

fn title(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    text(&buf, n).trim().to_string()
}

fn class(h: HWND) -> String {
    let mut buf = [0u16; 256];
    let n = unsafe { GetClassNameW(h, &mut buf) };
    text(&buf, n)
}

/// File name of the program that owns the window, like `game.exe`.
fn exe_name(pid: u32) -> String {
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        if !ok {
            return String::new();
        }
        let path = text(&buf, len as i32);
        path.rsplit(['\\', '/']).next().unwrap_or_default().to_string()
    }
}

/// Hidden store apps and shell parts that Windows keeps as invisible windows.
fn cloaked(h: HWND) -> bool {
    let mut value = 0u32;
    let ok = unsafe {
        DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut value as *mut u32 as *mut core::ffi::c_void, 4)
    };
    ok.is_ok() && value != 0
}

fn client_size(h: HWND) -> (i32, i32) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetClientRect(h, &mut r);
    }
    (r.right - r.left, r.bottom - r.top)
}

/// Where the picture inside the frame sits, in screen pixels.
fn client_rect(h: HWND) -> Rect {
    let (w, hh) = client_size(h);
    let mut origin = POINT::default();
    unsafe {
        let _ = ClientToScreen(h, &mut origin);
    }
    (origin.x, origin.y, w, hh)
}

fn window_rect(h: HWND) -> Rect {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(h, &mut r);
    }
    (r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// The screen the window is on (or the nearest one).
fn screen_of(h: HWND) -> Rect {
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let hmon = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
        let _ = GetMonitorInfoW(hmon, &mut info);
    }
    let r = info.rcMonitor;
    (r.left, r.top, r.right - r.left, r.bottom - r.top)
}

unsafe extern "system" fn collect(h: HWND, data: LPARAM) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<isize>);
    list.push(h.0 as isize);
    BOOL(1)
}

/// The window as the list shows it, or None when it is not a normal program window.
fn describe(raw: isize, me: u32, saved: &HashMap<isize, Saved>) -> Option<WindowInfo> {
    let h = hwnd(raw);
    unsafe {
        if !IsWindowVisible(h).as_bool() {
            return None;
        }
        if GetWindow(h, GW_OWNER).is_ok_and(|o| !o.0.is_null()) {
            return None;
        }
        if GetWindowLongPtrW(h, GWL_EXSTYLE) & WS_EX_TOOLWINDOW.0 as isize != 0 {
            return None;
        }
    }
    let pid = pid_of(h);
    if pid == me || cloaked(h) {
        return None;
    }
    if matches!(class(h).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        return None;
    }
    let title = title(h);
    if title.is_empty() {
        return None;
    }
    let (width, height) = if unsafe { IsIconic(h).as_bool() } { (0, 0) } else { client_size(h) };
    Some(WindowInfo {
        id: raw as i64,
        title,
        exe: exe_name(pid),
        width,
        height,
        borderless: saved.contains_key(&raw),
        framed: framed(h),
    })
}

/// Every normal program window, in the order Windows stacks them (front first).
pub fn list() -> Vec<WindowInfo> {
    let mut raw: Vec<isize> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut raw as *mut Vec<isize> as isize));
    }
    let me = std::process::id();
    let mut saved = saved();
    saved.retain(|h, _| alive(hwnd(*h)));
    raw.into_iter().filter_map(|h| describe(h, me, &saved)).collect()
}

/// Takes the frame off and moves the window where `fit` says.
pub fn apply(id: i64, fit: &Fit, monitors: &[MonitorInfo]) -> Result<(), String> {
    let raw = id as isize;
    let h = hwnd(raw);
    if !alive(h) {
        return Err(GONE.into());
    }
    if pid_of(h) == std::process::id() {
        return Err("That is Luna itself.".into());
    }
    let (style, ex) = unsafe { (GetWindowLongPtrW(h, GWL_STYLE), GetWindowLongPtrW(h, GWL_EXSTYLE)) };
    let first = !saved().contains_key(&raw);
    if first {
        saved().insert(raw, Saved { style, ex, rect: window_rect(h) });
    }

    let screen = monitors
        .iter()
        .find(|m| !fit.monitor.is_empty() && m.id == fit.monitor)
        .map(|m| (m.x, m.y, m.width, m.height))
        .unwrap_or_else(|| screen_of(h));
    let (x, y, w, hh) = fit::target(fit, screen, client_rect(h));

    unsafe {
        // A maximised or minimised window ignores a new size until it is a normal window again.
        if style & WS_MAXIMIZE.0 as isize != 0 || IsIconic(h).as_bool() {
            let _ = ShowWindowAsync(h, SW_RESTORE);
        }
        SetLastError(WIN32_ERROR(0));
        let changed = SetWindowLongPtrW(h, GWL_STYLE, style & !frame_bits());
        let err = GetLastError();
        if changed == 0 && err != WIN32_ERROR(0) {
            if first {
                saved().remove(&raw);
            }
            return Err(if err == ERROR_ACCESS_DENIED { ADMIN.into() } else { format!("Windows refused the change ({}).", err.0) });
        }
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex & !edge_bits());
        SetWindowPos(h, Some(HWND_TOP), x, y, w, hh, SWP_FRAMECHANGED | SWP_NOOWNERZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS)
            .map_err(|e| e.message())?;
    }
    Ok(())
}

/// Puts back the frame and place the window had before Luna changed it.
pub fn restore(id: i64) -> Result<(), String> {
    let raw = id as isize;
    let s = saved().remove(&raw).ok_or("Luna has not changed this window.")?;
    let h = hwnd(raw);
    if !alive(h) {
        return Err(GONE.into());
    }
    let (x, y, w, hh) = s.rect;
    unsafe {
        SetWindowLongPtrW(h, GWL_STYLE, s.style);
        SetWindowLongPtrW(h, GWL_EXSTYLE, s.ex);
        SetWindowPos(
            h,
            None,
            x,
            y,
            w,
            hh,
            SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
        .map_err(|e| e.message())?;
    }
    Ok(())
}

/// The fit for a program: its own when it is remembered, the page's otherwise.
fn fit_for<'a>(b: &'a Borderless, exe: &str) -> &'a Fit {
    b.games.iter().find(|g| !exe.is_empty() && g.exe.eq_ignore_ascii_case(exe)).map(|g| &g.fit).unwrap_or(&b.fit)
}

/// The key: switches the window in front between borderless and its own frame.
pub fn toggle_front(b: &Borderless, monitors: &[MonitorInfo]) -> Result<(), String> {
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() || pid_of(h) == std::process::id() {
        return Err("Switch to the game first, then press the key.".into());
    }
    let raw = h.0 as isize;
    if saved().contains_key(&raw) {
        return restore(raw as i64);
    }
    apply(raw as i64, fit_for(b, &exe_name(pid_of(h))), monitors)
}

/// Makes remembered games borderless a moment after their window opens, and again when a
/// game puts its frame back (some do on a resolution change). `current` reads the settings.
pub fn watch(current: impl Fn() -> Option<(Borderless, Vec<MonitorInfo>)> + Send + 'static) {
    std::thread::spawn(move || {
        // Window -> how many checks it has been seen; done = handled once already.
        let mut seen: HashMap<i64, u32> = HashMap::new();
        let mut done: HashSet<i64> = HashSet::new();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let Some((b, monitors)) = current() else { continue };
            if b.games.is_empty() {
                seen.clear();
                continue;
            }
            let windows = list();
            let open: HashSet<i64> = windows.iter().map(|w| w.id).collect();
            seen.retain(|id, _| open.contains(id));
            done.retain(|id| open.contains(id));
            for w in windows {
                if w.exe.is_empty() || !b.games.iter().any(|g| g.exe.eq_ignore_ascii_case(&w.exe)) {
                    continue;
                }
                let fit = fit_for(&b, &w.exe);
                if done.contains(&w.id) {
                    if w.borderless && w.framed {
                        let _ = apply(w.id, fit, &monitors);
                    }
                    continue;
                }
                let n = seen.entry(w.id).or_insert(0);
                *n += 1;
                // Seen twice: the game has finished opening its window. Splash screens
                // and launchers without a frame are left alone.
                if *n >= 2 {
                    done.insert(w.id);
                    if w.framed && !w.borderless {
                        let _ = apply(w.id, fit, &monitors);
                    }
                }
            }
        }
    });
}
