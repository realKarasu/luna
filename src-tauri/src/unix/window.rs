//! Overlay windows (crosshair and look wheel) through Tauri. Called from the engine thread,
//! the setters are posted to the event loop and return at once.

use std::sync::atomic::Ordering;
use tauri::WebviewWindow;

pub struct Overlay(WebviewWindow);

impl Overlay {
    pub fn from_window(w: &WebviewWindow) -> Option<Overlay> {
        Some(Overlay(w.clone()))
    }

    /// Position and size in the units of the monitor list: points on macOS, pixels on Linux.
    pub fn place(&self, x: i32, y: i32, w: i32, h: i32, visible: bool) {
        if !visible {
            let _ = self.0.hide();
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let _ = self.0.set_size(tauri::LogicalSize::new(w as f64, h as f64));
            let _ = self.0.set_position(tauri::LogicalPosition::new(x as f64, y as f64));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = self.0.set_size(tauri::PhysicalSize::new(w.max(1) as u32, h.max(1) as u32));
            let _ = self.0.set_position(tauri::PhysicalPosition::new(x, y));
        }
        let _ = self.0.show();
        let _ = self.0.set_always_on_top(true);
    }
}

/// Where the mouse pointer was at the last input poll.
pub fn cursor_pos() -> (i32, i32) {
    super::hooks::cursor()
}

/// True while Luna's own window has the focus, so mouse binds do not fire on its buttons.
pub fn luna_in_front() -> bool {
    crate::input::MAIN_FOCUSED.load(Ordering::Relaxed)
}
