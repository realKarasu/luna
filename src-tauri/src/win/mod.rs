//! Everything that talks to Windows.

pub mod adl;
pub mod borderless;
pub mod display;
pub mod gpu;
pub mod hooks;
pub mod mag;
pub mod nvapi;
pub mod optimizer;
pub mod vib;
pub mod window;

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "Luna";

/// Start with Windows through the per-user Run key.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(RUN_NAME, &format!("\"{}\" --autostart", exe.display())).map_err(|e| e.to_string())
    } else {
        match key.delete_value(RUN_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_NAME))
        .is_ok()
}

/// Opens a folder in Explorer.
pub fn open_dir(dir: &std::path::Path) {
    let _ = std::process::Command::new("explorer.exe").arg(dir).spawn();
}

/// Something the user should know about this system, shown in the app.
pub fn note() -> Option<String> {
    None
}
