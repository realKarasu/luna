//! Everything that talks to Windows.

pub mod adl;
pub mod borderless;
pub mod cleaner;
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

/// Starts Luna again as administrator (Windows asks first). The caller then quits this copy.
pub fn relaunch_admin(page: &str) -> Result<(), String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let page = if !page.is_empty() && page.chars().all(|c| c.is_ascii_alphanumeric()) { page } else { "display" };
    let args = HSTRING::from(format!("--after {} --page {page}", std::process::id()));
    let done = unsafe { ShellExecuteW(None, w!("runas"), &HSTRING::from(exe.as_os_str()), &args, None, SW_SHOWNORMAL) };
    // Anything above 32 means it started. Five is a refused prompt.
    match done.0 as isize {
        n if n > 32 => Ok(()),
        5 => Err("Windows did not get permission to start Luna as administrator.".into()),
        n => Err(format!("Luna could not start as administrator (error {n}).")),
    }
}

/// Waits a few seconds for the copy of Luna that started this one to close.
pub fn wait_for_exit(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, 5000);
            let _ = CloseHandle(h);
        }
    }
}
