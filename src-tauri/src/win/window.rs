//! Raw window helpers for the overlays. Every call is asynchronous so the engine
//! thread never waits on the UI thread.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowThreadProcessId, SetWindowLongPtrW, SetWindowPos,
    ShowWindowAsync, GWL_EXSTYLE, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_HIDE,
    SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// Makes a window an overlay: never focused, no taskbar button, always on top.
/// Click-through itself comes from Tauri's `set_ignore_cursor_events`.
pub fn make_overlay(raw: isize) {
    unsafe {
        let h = hwnd(raw);
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let add = (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST).0 as isize;
        SetWindowLongPtrW(h, GWL_EXSTYLE, ex | add);
    }
}

/// Moves, sizes, raises and shows (or hides) an overlay without activating it.
pub fn place(raw: isize, x: i32, y: i32, w: i32, h: i32, visible: bool) {
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
