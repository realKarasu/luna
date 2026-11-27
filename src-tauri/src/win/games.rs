//! What the game profiles need from Windows: the running programs, the program in front
//! and the screen modes. A mode Luna sets is temporary: Windows forgets it at sign-out,
//! and Luna puts the saved one back when the game closes, at quit and after a crash.

use crate::settings::DisplayMode;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex, TryLockError};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplaySettingsW, CDS_FULLSCREEN, CDS_TYPE, DEVMODEW, DISP_CHANGE_SUCCESSFUL,
    DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE,
    ENUM_REGISTRY_SETTINGS,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Screens whose mode Luna changed, so they can be put back.
static CHANGED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Every running program as (process id, lower-case file name).
pub fn processes() -> Vec<(u32, String)> {
    let mut list = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return list };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut entry).is_ok();
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            list.push((entry.th32ProcessID, String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase()));
            ok = Process32NextW(snap, &mut entry).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    list
}

/// Lower-case file names of every running program.
pub fn running() -> HashSet<String> {
    processes().into_iter().map(|(_, exe)| exe).collect()
}

/// The process that owns the window in front, 0 when there is none.
pub fn foreground_pid() -> u32 {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return 0;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        pid
    }
}

/// Lower-case file name of a process, empty when Windows does not say.
pub fn exe_of(pid: u32) -> String {
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        if !ok {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().unwrap_or_default().to_lowercase()
    }
}

fn read_mode(device: &str, index: ENUM_DISPLAY_SETTINGS_MODE) -> Option<DEVMODEW> {
    let name = wide(device);
    let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
    unsafe { EnumDisplaySettingsW(PCWSTR(name.as_ptr()), index, &mut dm).as_bool().then_some(dm) }
}

fn mode_of(dm: &DEVMODEW) -> DisplayMode {
    DisplayMode { width: dm.dmPelsWidth as i32, height: dm.dmPelsHeight as i32, hz: dm.dmDisplayFrequency as i32 }
}

/// The modes a screen offers in full colour, largest and fastest first.
pub fn modes(device: &str) -> Vec<DisplayMode> {
    let mut seen = HashSet::new();
    let mut list = Vec::new();
    let mut i = 0u32;
    while let Some(dm) = read_mode(device, ENUM_DISPLAY_SETTINGS_MODE(i)) {
        i += 1;
        let m = mode_of(&dm);
        if dm.dmBitsPerPel == 32 && m.width >= 640 && m.hz > 1 && seen.insert(m) {
            list.push(m);
        }
        if i > 4096 {
            break;
        }
    }
    list.sort_by_key(|m| std::cmp::Reverse((m.width * m.height, m.width, m.hz)));
    list
}

/// Switches a screen to a mode until `restore_modes`. Nothing happens when it already shows it.
/// `settle`: another screen just changed, so the driver gets two seconds to offer the mode.
pub fn set_mode(device: &str, mode: &DisplayMode, settle: bool) -> Result<(), String> {
    let mut dm = read_mode(device, ENUM_CURRENT_SETTINGS).ok_or("Windows did not describe that screen.")?;
    if mode_of(&dm) == *mode {
        return Ok(());
    }
    let mut offered = modes(device).contains(mode);
    let mut tries = 0;
    while !offered && settle && tries < 8 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        offered = modes(device).contains(mode);
        tries += 1;
    }
    if !offered {
        return Err(format!(
            "The screen does not offer {} x {} at {} Hz right now. The profile left it as it was.",
            mode.width, mode.height, mode.hz
        ));
    }
    dm.dmPelsWidth = mode.width as u32;
    dm.dmPelsHeight = mode.height as u32;
    dm.dmDisplayFrequency = mode.hz as u32;
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
    let name = wide(device);
    let r = unsafe { ChangeDisplaySettingsExW(PCWSTR(name.as_ptr()), Some(&dm), None, CDS_FULLSCREEN, None) };
    if r != DISP_CHANGE_SUCCESSFUL {
        return Err(format!("Windows refused the screen mode (code {}).", r.0));
    }
    CHANGED.lock().unwrap_or_else(|e| e.into_inner()).insert(device.to_string());
    Ok(())
}

/// Puts back the mode saved in Windows settings, whoever changed it. Used after a crash.
pub fn reset_mode(device: &str) {
    put_back(device);
}

fn put_back(device: &str) {
    let name = wide(device);
    // No mode and no flags: the mode saved in Windows settings comes back.
    unsafe {
        let _ = ChangeDisplaySettingsExW(PCWSTR(name.as_ptr()), None, None, CDS_TYPE(0), None);
    }
}

/// The screen shows the mode saved in Windows settings.
fn is_back(device: &str) -> bool {
    match (read_mode(device, ENUM_CURRENT_SETTINGS), read_mode(device, ENUM_REGISTRY_SETTINGS)) {
        (Some(now), Some(saved)) => mode_of(&now) == mode_of(&saved),
        _ => true,
    }
}

/// Puts back the screens' own modes, in this order, if Luna changed them. A screen can
/// only take its fast mode back once another one has slowed down, so it tries a few times.
pub fn restore_modes(devices: &[String]) {
    let mine: Vec<&String> = {
        let mut changed = CHANGED.lock().unwrap_or_else(|e| e.into_inner());
        devices.iter().filter(|d| changed.remove(*d)).collect()
    };
    for round in 0..4 {
        let left: Vec<&&String> = mine.iter().filter(|d| round == 0 || !is_back(d)).collect();
        if left.is_empty() {
            break;
        }
        if round > 0 {
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        for d in left {
            put_back(d);
        }
    }
}

/// Every screen Luna changed back to its own mode. Safe from the panic hook.
pub fn restore_all_modes() {
    let devices: Vec<String> = match CHANGED.try_lock() {
        Ok(mut g) => g.drain().collect(),
        Err(TryLockError::Poisoned(p)) => p.into_inner().drain().collect(),
        Err(TryLockError::WouldBlock) => return,
    };
    for d in devices {
        put_back(&d);
    }
}
