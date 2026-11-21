//! Input events and key names shared by every platform. Keys are always Windows
//! virtual-key codes, so saved binds mean the same thing everywhere.

use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    Down(u32),
    Up(u32),
    MouseDelta(i32, i32),
    /// The system changed the screen layout, a resolution or a graphics card.
    #[allow(dead_code)]
    DisplayChanged,
}

/// Set while the look wheel is open, so mouse moves are forwarded.
pub static TRACK_MOUSE: AtomicBool = AtomicBool::new(false);

/// True while Luna's main window has the keyboard focus.
pub static MAIN_FOCUSED: AtomicBool = AtomicBool::new(false);

/// A readable name for a virtual-key code, when it has a fixed one.
pub fn basic_label(vk: u32) -> Option<String> {
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
        0x12 | 0xA4 => Some(if cfg!(target_os = "macos") { "Option" } else { "Alt" }),
        0xA5 => Some(if cfg!(target_os = "macos") { "Right Option" } else { "Right Alt" }),
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
        0x5B => Some(if cfg!(target_os = "macos") { "Command" } else if cfg!(windows) { "Left Win" } else { "Super" }),
        0x5C => Some(if cfg!(target_os = "macos") { "Right Command" } else if cfg!(windows) { "Right Win" } else { "Right Super" }),
        0x6A => Some("Num *"),
        0x6B => Some("Num +"),
        0x6D => Some("Num -"),
        0x6E => Some("Num ."),
        0x6F => Some("Num /"),
        0x90 => Some("Num Lock"),
        0x91 => Some("Scroll Lock"),
        0xBA => Some(";"),
        0xBB => Some("="),
        0xBC => Some(","),
        0xBD => Some("-"),
        0xBE => Some("."),
        0xBF => Some("/"),
        0xC0 => Some("`"),
        0xDB => Some("["),
        0xDC => Some("\\"),
        0xDD => Some("]"),
        0xDE => Some("'"),
        _ => None,
    };
    if let Some(f) = fixed {
        return Some(f.to_string());
    }
    if (0x70..=0x87).contains(&vk) {
        return Some(format!("F{}", vk - 0x6F));
    }
    if (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) {
        return char::from_u32(vk).map(|c| c.to_string());
    }
    if (0x60..=0x69).contains(&vk) {
        return Some(format!("Num {}", vk - 0x60));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(basic_label(0x41).as_deref(), Some("A"));
        assert_eq!(basic_label(0x76).as_deref(), Some("F7"));
        assert_eq!(basic_label(0x63).as_deref(), Some("Num 3"));
        assert_eq!(basic_label(5).as_deref(), Some("Mouse 4"));
        assert_eq!(basic_label(0xFF), None);
    }
}
