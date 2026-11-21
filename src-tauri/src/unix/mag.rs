//! The full-screen colour matrix and Ultra Zoom come from the Windows magnifier, which has no
//! counterpart Luna can drive here. Hue, saturation and zoom are left alone; the app says so.

pub fn start() {}

pub fn available() -> bool {
    false
}

pub fn set_color(_m: [f32; 25]) {}

pub fn set_zoom(_level: f32, _x: i32, _y: i32) {}

pub fn shutdown() {}
