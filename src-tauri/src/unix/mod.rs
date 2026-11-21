//! What macOS and Linux share: input through polling, overlay windows through Tauri, and the
//! Windows-only features (colour matrix, zoom, optimizer, borderless windows, cleaning) as empty stand-ins.

pub mod borderless;
pub mod cleaner;
pub mod hooks;
pub mod mag;
pub mod optimizer;
pub mod window;

/// Opens a folder in the file manager.
pub fn open_dir(dir: &std::path::Path) {
    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(opener).arg(dir).spawn();
}

/// The program to start at login: the AppImage when Luna runs from one, else this binary.
pub fn launcher() -> Result<std::path::PathBuf, String> {
    if let Some(p) = std::env::var_os("APPIMAGE").filter(|p| !p.is_empty()) {
        return Ok(p.into());
    }
    std::env::current_exe().map_err(|e| e.to_string())
}

pub fn relaunch_admin(_page: &str) -> Result<(), String> {
    Err("Only Luna on Windows can restart as administrator.".into())
}

pub fn wait_for_exit(_pid: u32) {}
