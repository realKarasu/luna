//! Linux (X11): gamma tables through XRandR, NVIDIA vibrance through `nvidia-settings`,
//! input through polling the X server and start at login through an XDG autostart entry.
//! There is no colour matrix, zoom or optimizer here.

pub mod display;
pub mod vib;

pub use crate::unix::{
    borderless, cleaner, fps, games, hooks, library, mag, open_dir, optimizer, potato, sysmon, relaunch_admin, wait_for_exit, window,
};

use std::path::PathBuf;

pub fn note() -> Option<String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    wayland.then(|| {
        "Wayland session: screen colour and hotkeys only work in an X11 session. Pick \"on Xorg\" at the login screen to use them.".into()
    })
}

fn entry_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("luna.desktop"))
}

/// Quotes a path for the Exec key of a desktop entry.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        if matches!(ch, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// Start at login through `~/.config/autostart/luna.desktop`.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let path = entry_path().ok_or("No home folder")?;
    if !on {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    let exe = crate::unix::launcher()?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Luna\nComment=Display tuning for games\nExec={} --autostart\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        quote(&exe.display().to_string())
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, entry).map_err(|e| e.to_string())
}

pub fn autostart_enabled() -> bool {
    entry_path().is_some_and(|p| p.exists())
}

#[cfg(test)]
mod tests {
    #[test]
    fn quotes_exec() {
        assert_eq!(super::quote("/opt/My Luna/luna"), "\"/opt/My Luna/luna\"");
        assert_eq!(super::quote("/a\"b"), "\"/a\\\"b\"");
    }
}
