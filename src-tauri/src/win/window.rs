//! Raw window helpers for the overlays. Every call is asynchronous so the engine
//! thread never waits on the UI thread.

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, SetWindowLongPtrW, SetWindowPos,
    ShowWindowAsync, GWL_EXSTYLE, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// An overlay window (crosshair or look wheel), kept as its raw handle.
pub struct Overlay(isize);

impl Overlay {
    pub fn from_window(w: &tauri::WebviewWindow) -> Option<Overlay> {
        let raw = w.hwnd().ok()?.0 as isize;
        if raw == 0 {
            return None;
        }
        make_overlay(raw);
        Some(Overlay(raw))
    }

    /// Position and size in physical pixels, like the monitor list.
    pub fn place(&self, x: i32, y: i32, w: i32, h: i32, visible: bool) {
        place(self.0, x, y, w, h, visible);
    }
}

/// Where the mouse pointer is, in screen pixels.
pub fn cursor_pos() -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    (pt.x, pt.y)
}

/// Makes a window an overlay: never focused, no taskbar button, always on top.
/// Click-through itself comes from Tauri's `set_ignore_cursor_events`.
fn make_overlay(raw: isize) {
    unsafe {
        let h = hwnd(raw);
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let add = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize;
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex | add);
    }
}

/// Moves, sizes, raises and shows (or hides) an overlay without activating it.
fn place(raw: isize, x: i32, y: i32, w: i32, h: i32, visible: bool) {
    unsafe {
        let handle = hwnd(raw);
        if visible {
            let _ = SetWindowPos(
                handle,
                Some(HWND_TOPMOST),
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS | SWP_NOOWNERZORDER,
            );
            let _ = ShowWindowAsync(handle, SW_SHOWNOACTIVATE);
        } else {
            let _ = ShowWindowAsync(handle, SW_HIDE);
        }
    }
}

/// True when the window in front belongs to Luna itself.
pub fn luna_in_front() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        pid == std::process::id()
    }
}
