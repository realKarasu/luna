//! macOS: gamma tables through CoreGraphics, input through polling (needs the Accessibility
//! permission) and start at login through a LaunchAgent. There is no driver vibrance, colour
//! matrix, zoom or optimizer here.

pub mod display;
pub mod vib;

pub use crate::unix::{hooks, mag, open_dir, optimizer, window};

use std::path::PathBuf;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

/// Luna may read keys and buttons (Privacy & Security > Accessibility).
pub fn accessibility_granted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

pub fn note() -> Option<String> {
    if accessibility_granted() {
        None
    } else {
        Some(
            "Hotkeys and mouse binds need the Accessibility permission: System Settings > Privacy & Security > Accessibility, then turn Luna on.".into(),
        )
    }
}

const AGENT: &str = "app.luna.displaytuner";

fn agent_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(PathBuf::from(home).join("Library").join("LaunchAgents").join(format!("{AGENT}.plist")))
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Start at login through a per-user LaunchAgent.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let path = agent_path().ok_or("No home folder")?;
    if !on {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    let exe = crate::unix::launcher()?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{AGENT}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>--autostart</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#,
        xml(&exe.display().to_string())
    );
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, plist).map_err(|e| e.to_string())
}

pub fn autostart_enabled() -> bool {
    agent_path().is_some_and(|p| p.exists())
}
