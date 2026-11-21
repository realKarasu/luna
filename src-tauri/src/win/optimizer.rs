//! Windows gaming tweaks. Per-user (HKCU) changes and per-user system settings only.
//! Each switch reads the live state, backs up the previous values to
//! %APPDATA%\Luna\optimizer-backup.json before changing anything, and can undo.

use crate::settings::{data_dir, write_json_atomic};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{LocalFree, HLOCAL, LPARAM, WPARAM};
use windows::Win32::System::Power::{PowerGetActiveScheme, PowerSetActiveScheme};
use windows::Win32::UI::Accessibility::{STICKYKEYS, STICKYKEYS_FLAGS};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, SystemParametersInfoW, ANIMATIONINFO, HWND_BROADCAST, SMTO_ABORTIFHUNG, SPIF_SENDCHANGE,
    SPIF_UPDATEINIFILE, SPI_GETANIMATION, SPI_GETMOUSE, SPI_GETSTICKYKEYS, SPI_SETANIMATION, SPI_SETMOUSE,
    SPI_SETSTICKYKEYS, WM_SETTINGCHANGE,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
use winreg::RegKey;

pub const IDS: [&str; 10] = [
    "power",
    "gameMode",
    "gameDvr",
    "flipModel",
    "mouseAccel",
    "stickyKeys",
    "notifications",
    "backgroundApps",
    "transparency",
    "animations",
];

const HIGH_PERFORMANCE: u128 = 0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c;
const BALANCED: u128 = 0x381b4222_f694_41f0_9685_ff5bb260df2e;
const SKF_HOTKEYACTIVE: u32 = 0x0000_0004;
const SKF_CONFIRMHOTKEY: u32 = 0x0000_0008;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegData {
    Dword { value: u32 },
    Sz { value: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BackupItem {
    Reg { path: String, name: String, prev: Option<RegData> },
    Power { guid: String },
    Mouse { values: [i32; 3] },
    Sticky { flags: u32 },
    Animation { min_animate: i32 },
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct BackupFile {
    items: HashMap<String, Vec<BackupItem>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakStatus {
    pub id: String,
    pub on: bool,
    pub available: bool,
    pub backed_up: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub freed_bytes: u64,
    pub files: u64,
    pub skipped: u64,
}

fn backup_path() -> std::path::PathBuf {
    data_dir().join("optimizer-backup.json")
}

fn load_backup() -> BackupFile {
    std::fs::read_to_string(backup_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_backup(b: &BackupFile) {
    let _ = write_json_atomic(&backup_path(), b);
}

// ---------- registry helpers ----------

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

fn reg_read(path: &str, name: &str) -> Option<RegData> {
    let key = hkcu().open_subkey_with_flags(path, KEY_READ).ok()?;
    if let Ok(v) = key.get_value::<u32, _>(name) {
        return Some(RegData::Dword { value: v });
    }
    if let Ok(v) = key.get_value::<String, _>(name) {
        return Some(RegData::Sz { value: v });
    }
    None
}

fn reg_dword(path: &str, name: &str) -> Option<u32> {
    match reg_read(path, name) {
        Some(RegData::Dword { value }) => Some(value),
        Some(RegData::Sz { value }) => value.trim().parse().ok(),
        None => None,
    }
}

fn reg_write(path: &str, name: &str, data: &RegData) -> Result<(), String> {
    let (key, _) = hkcu().create_subkey(path).map_err(|e| e.to_string())?;
    match data {
        RegData::Dword { value } => key.set_value(name, value),
        RegData::Sz { value } => key.set_value(name, value),
    }
    .map_err(|e| e.to_string())
}

fn reg_delete(path: &str, name: &str) {
    if let Ok((key, _)) = hkcu().create_subkey(path) {
        let _ = key.delete_value(name);
    }
}

/// Backs up then writes a registry value.
fn reg_set_backed(backup: &mut Vec<BackupItem>, path: &str, name: &str, data: RegData) -> Result<(), String> {
    backup.push(BackupItem::Reg { path: path.into(), name: name.into(), prev: reg_read(path, name) });
    reg_write(path, name, &data)
}

fn broadcast(area: &str) {
    let text: Vec<u16> = area.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(PCWSTR(text.as_ptr()).0 as isize),
            SMTO_ABORTIFHUNG,
            500,
            None,
        );
    }
}

// ---------- system parameter helpers ----------

fn get_mouse() -> Option<[i32; 3]> {
    let mut v = [0i32; 3];
    unsafe { SystemParametersInfoW(SPI_GETMOUSE, 0, Some(v.as_mut_ptr() as *mut _), Default::default()).ok()? };
    Some(v)
}

fn set_mouse(mut v: [i32; 3]) -> Result<(), String> {
    unsafe {
        SystemParametersInfoW(SPI_SETMOUSE, 0, Some(v.as_mut_ptr() as *mut _), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE)
            .map_err(|e| e.to_string())
    }
}

fn get_sticky() -> Option<u32> {
    let mut sk = STICKYKEYS { cbSize: std::mem::size_of::<STICKYKEYS>() as u32, dwFlags: STICKYKEYS_FLAGS(0) };
    unsafe {
        SystemParametersInfoW(SPI_GETSTICKYKEYS, sk.cbSize, Some(&mut sk as *mut _ as *mut _), Default::default()).ok()?
    };
    Some(sk.dwFlags.0)
}

fn set_sticky(flags: u32) -> Result<(), String> {
    let mut sk = STICKYKEYS { cbSize: std::mem::size_of::<STICKYKEYS>() as u32, dwFlags: STICKYKEYS_FLAGS(flags) };
    unsafe {
        SystemParametersInfoW(
            SPI_SETSTICKYKEYS,
            sk.cbSize,
            Some(&mut sk as *mut _ as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
        .map_err(|e| e.to_string())
    }
}

fn get_animation() -> Option<i32> {
    let mut ai = ANIMATIONINFO { cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32, iMinAnimate: 0 };
    unsafe {
        SystemParametersInfoW(SPI_GETANIMATION, ai.cbSize, Some(&mut ai as *mut _ as *mut _), Default::default()).ok()?
    };
    Some(ai.iMinAnimate)
}

fn set_animation(on: i32) -> Result<(), String> {
    let mut ai = ANIMATIONINFO { cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32, iMinAnimate: on };
    unsafe {
        SystemParametersInfoW(
            SPI_SETANIMATION,
            ai.cbSize,
            Some(&mut ai as *mut _ as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
        .map_err(|e| e.to_string())
    }
}

// ---------- power plans ----------

fn guid_string(g: &GUID) -> String {
    format!("{:?}", g).to_lowercase()
}

fn parse_guid(s: &str) -> Option<GUID> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 32 {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok().map(GUID::from_u128)
}

fn find_guid_in(text: &str) -> Option<GUID> {
    text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .find(|w| w.len() == 36 && w.matches('-').count() == 4)
        .and_then(parse_guid)
}

fn active_scheme() -> Option<GUID> {
    unsafe {
        let mut p: *mut GUID = std::ptr::null_mut();
        if PowerGetActiveScheme(None, &mut p).is_err() || p.is_null() {
            return None;
        }
        let g = *p;
        let _ = LocalFree(Some(HLOCAL(p as *mut _)));
        Some(g)
    }
}

fn set_scheme(g: &GUID) -> bool {
    unsafe { PowerSetActiveScheme(None, Some(g as *const GUID)).is_ok() }
}

fn powercfg(args: &[&str]) -> Option<String> {
    std::process::Command::new("powercfg")
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn high_perf_guids() -> Vec<GUID> {
    let mut v = vec![GUID::from_u128(HIGH_PERFORMANCE)];
    if let Some(extra) = load_backup().items.get("powerCopy").and_then(|i| i.first().cloned()) {
        if let BackupItem::Power { guid } = extra {
            if let Some(g) = parse_guid(&guid) {
                v.push(g);
            }
        }
    }
    v
}

// ---------- the switches ----------

const GAMEBAR: &str = r"Software\Microsoft\GameBar";
const GAMECONFIG: &str = r"System\GameConfigStore";
const GAMEDVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const GPUPREFS: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const PUSH: &str = r"Software\Microsoft\Windows\CurrentVersion\PushNotifications";
const BGAPPS: &str = r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications";
const SEARCH: &str = r"Software\Microsoft\Windows\CurrentVersion\Search";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

fn gpu_settings_has_flip() -> bool {
    match reg_read(GPUPREFS, "DirectXUserGlobalSettings") {
        Some(RegData::Sz { value }) => value.split(';').any(|p| p.trim() == "SwapEffectUpgradeEnable=1"),
        _ => false,
    }
}

/// Merges `SwapEffectUpgradeEnable=1` into the semicolon separated setting string.
pub fn merge_gpu_setting(existing: &str, key: &str, value: &str) -> String {
    let mut parts: Vec<String> = existing
        .split(';')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty() && !p.starts_with(&format!("{key}=")))
        .map(|p| p.to_string())
        .collect();
    parts.push(format!("{key}={value}"));
    let mut s = parts.join(";");
    s.push(';');
    s
}

fn is_on(id: &str) -> Option<bool> {
    Some(match id {
        "power" => {
            let a = active_scheme()?;
            high_perf_guids().contains(&a)
        }
        "gameMode" => reg_dword(GAMEBAR, "AutoGameModeEnabled").unwrap_or(1) == 1,
        "gameDvr" => {
            reg_dword(GAMECONFIG, "GameDVR_Enabled") == Some(0) && reg_dword(GAMEDVR, "AppCaptureEnabled") == Some(0)
        }
        "flipModel" => gpu_settings_has_flip(),
        "mouseAccel" => get_mouse()?[2] == 0,
        "stickyKeys" => get_sticky()? & SKF_HOTKEYACTIVE == 0,
        "notifications" => reg_dword(PUSH, "ToastEnabled") == Some(0),
        "backgroundApps" => reg_dword(BGAPPS, "GlobalUserDisabled") == Some(1),
        "transparency" => reg_dword(PERSONALIZE, "EnableTransparency") == Some(0),
        "animations" => get_animation()? == 0,
        _ => return None,
    })
}

fn apply(id: &str) -> Result<Vec<BackupItem>, String> {
    let mut b = Vec::new();
    match id {
        "power" => {
            let current = active_scheme().ok_or("Could not read the power plan.")?;
            b.push(BackupItem::Power { guid: guid_string(&current) });
            if !set_scheme(&GUID::from_u128(HIGH_PERFORMANCE)) {
                // Some PCs hide the plan: make a personal copy of it, then use that.
                let out = powercfg(&["/duplicatescheme", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"])
                    .ok_or("powercfg is not available.")?;
                let copy = find_guid_in(&out).ok_or("This PC has no High performance plan.")?;
                if !set_scheme(&copy) {
                    return Err("Windows refused to switch the power plan.".into());
                }
                let mut file = load_backup();
                file.items.insert("powerCopy".into(), vec![BackupItem::Power { guid: guid_string(&copy) }]);
                save_backup(&file);
            }
        }
        "gameMode" => {
            reg_set_backed(&mut b, GAMEBAR, "AutoGameModeEnabled", RegData::Dword { value: 1 })?;
            reg_set_backed(&mut b, GAMEBAR, "AllowAutoGameMode", RegData::Dword { value: 1 })?;
        }
        "gameDvr" => {
            reg_set_backed(&mut b, GAMECONFIG, "GameDVR_Enabled", RegData::Dword { value: 0 })?;
            reg_set_backed(&mut b, GAMEDVR, "AppCaptureEnabled", RegData::Dword { value: 0 })?;
            reg_set_backed(&mut b, GAMEDVR, "HistoricalCaptureEnabled", RegData::Dword { value: 0 })?;
        }
        "flipModel" => {
            let existing = match reg_read(GPUPREFS, "DirectXUserGlobalSettings") {
                Some(RegData::Sz { value }) => value,
                _ => String::new(),
            };
            let merged = merge_gpu_setting(&existing, "SwapEffectUpgradeEnable", "1");
            reg_set_backed(&mut b, GPUPREFS, "DirectXUserGlobalSettings", RegData::Sz { value: merged })?;
        }
        "mouseAccel" => {
            let current = get_mouse().ok_or("Could not read the mouse settings.")?;
            b.push(BackupItem::Mouse { values: current });
            set_mouse([0, 0, 0])?;
        }
        "stickyKeys" => {
            let flags = get_sticky().ok_or("Could not read the Sticky Keys settings.")?;
            b.push(BackupItem::Sticky { flags });
            set_sticky(flags & !(SKF_HOTKEYACTIVE | SKF_CONFIRMHOTKEY))?;
        }
        "notifications" => {
            reg_set_backed(&mut b, PUSH, "ToastEnabled", RegData::Dword { value: 0 })?;
        }
        "backgroundApps" => {
            reg_set_backed(&mut b, BGAPPS, "GlobalUserDisabled", RegData::Dword { value: 1 })?;
            reg_set_backed(&mut b, SEARCH, "BackgroundAppGlobalToggle", RegData::Dword { value: 0 })?;
        }
        "transparency" => {
            reg_set_backed(&mut b, PERSONALIZE, "EnableTransparency", RegData::Dword { value: 0 })?;
            broadcast("ImmersiveColorSet");
        }
        "animations" => {
            let current = get_animation().ok_or("Could not read the animation settings.")?;
            b.push(BackupItem::Animation { min_animate: current });
            set_animation(0)?;
        }
        _ => return Err("Unknown switch.".into()),
    }
    Ok(b)
}

fn restore_item(item: &BackupItem) {
    match item {
        BackupItem::Reg { path, name, prev } => match prev {
            Some(data) => {
                let _ = reg_write(path, name, data);
            }
            None => reg_delete(path, name),
        },
        BackupItem::Power { guid } => {
            if let Some(g) = parse_guid(guid) {
                if !set_scheme(&g) {
                    set_scheme(&GUID::from_u128(BALANCED));
                }
            }
        }
        BackupItem::Mouse { values } => {
            let _ = set_mouse(*values);
        }
        BackupItem::Sticky { flags } => {
            let _ = set_sticky(*flags);
        }
        BackupItem::Animation { min_animate } => {
            let _ = set_animation(*min_animate);
        }
    }
}

/// Windows defaults, used when a switch is turned off but no backup exists
/// (for example it was changed before Luna was installed).
fn restore_default(id: &str) {
    match id {
        "power" => {
            set_scheme(&GUID::from_u128(BALANCED));
        }
        "gameMode" => {} // Game Mode on is already the Windows default.
        "gameDvr" => {
            let _ = reg_write(GAMECONFIG, "GameDVR_Enabled", &RegData::Dword { value: 1 });
            reg_delete(GAMEDVR, "AppCaptureEnabled");
            reg_delete(GAMEDVR, "HistoricalCaptureEnabled");
        }
        "flipModel" => {
            if let Some(RegData::Sz { value }) = reg_read(GPUPREFS, "DirectXUserGlobalSettings") {
                let rest: Vec<&str> = value
                    .split(';')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty() && !p.starts_with("SwapEffectUpgradeEnable="))
                    .collect();
                if rest.is_empty() {
                    reg_delete(GPUPREFS, "DirectXUserGlobalSettings");
                } else {
                    let _ = reg_write(GPUPREFS, "DirectXUserGlobalSettings", &RegData::Sz { value: format!("{};", rest.join(";")) });
                }
            }
        }
        "mouseAccel" => {
            let _ = set_mouse([6, 10, 1]);
        }
        "stickyKeys" => {
            if let Some(f) = get_sticky() {
                let _ = set_sticky(f | SKF_HOTKEYACTIVE | SKF_CONFIRMHOTKEY);
            }
        }
        "notifications" => reg_delete(PUSH, "ToastEnabled"),
        "backgroundApps" => {
            reg_delete(BGAPPS, "GlobalUserDisabled");
            reg_delete(SEARCH, "BackgroundAppGlobalToggle");
        }
        "transparency" => {
            let _ = reg_write(PERSONALIZE, "EnableTransparency", &RegData::Dword { value: 1 });
            broadcast("ImmersiveColorSet");
        }
        "animations" => {
            let _ = set_animation(1);
        }
        _ => {}
    }
}

pub fn status() -> Vec<TweakStatus> {
    let backup = load_backup();
    IDS.iter()
        .map(|id| {
            let on = is_on(id);
            TweakStatus {
                id: id.to_string(),
                on: on.unwrap_or(false),
                available: on.is_some(),
                backed_up: backup.items.contains_key(*id),
            }
        })
        .collect()
}

/// Turns one switch on (applies the tweak) or off (undoes it).
pub fn set(id: &str, on: bool) -> Result<(), String> {
    if !IDS.contains(&id) {
        return Err("Unknown switch.".into());
    }
    let mut file = load_backup();
    if on {
        if is_on(id) == Some(true) {
            return Ok(());
        }
        let items = apply(id)?;
        // Keep the oldest backup: it holds the value from before Luna touched it.
        file.items.entry(id.to_string()).or_insert(items);
        save_backup(&file);
    } else {
        match file.items.remove(id) {
            Some(items) => {
                for item in items.iter().rev() {
                    restore_item(item);
                }
                if id == "transparency" {
                    broadcast("ImmersiveColorSet");
                }
            }
            None => restore_default(id),
        }
        save_backup(&file);
    }
    Ok(())
}

pub fn optimize_all() -> Vec<String> {
    IDS.iter().filter_map(|id| set(id, true).err().map(|e| format!("{id}: {e}"))).collect()
}

pub fn undo_all() -> Vec<String> {
    let file = load_backup();
    IDS.iter()
        .filter(|id| file.items.contains_key(**id))
        .filter_map(|id| set(id, false).err().map(|e| format!("{id}: {e}")))
        .collect()
}

/// Deletes files older than one day from the user's %TEMP% folder.
pub fn clean_temp() -> CleanResult {
    let mut res = CleanResult { freed_bytes: 0, files: 0, skipped: 0 };
    let root = std::env::temp_dir();
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
            let ft = meta.file_type();
            if ft.is_symlink() {
                continue; // never follow links out of %TEMP%
            }
            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            let old = meta.modified().map(|m| m < cutoff).unwrap_or(false);
            if !old {
                continue;
            }
            let size = meta.len();
            if std::fs::remove_file(&path).is_ok() {
                res.freed_bytes += size;
                res.files += 1;
            } else {
                res.skipped += 1;
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_into_empty() {
        assert_eq!(merge_gpu_setting("", "SwapEffectUpgradeEnable", "1"), "SwapEffectUpgradeEnable=1;");
    }

    #[test]
    fn merge_keeps_other_settings_and_replaces_old_value() {
        let s = merge_gpu_setting("VRROptimizeEnable=0;SwapEffectUpgradeEnable=0;", "SwapEffectUpgradeEnable", "1");
        assert_eq!(s, "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;");
    }

    #[test]
    fn guid_parse_roundtrip() {
        let g = GUID::from_u128(HIGH_PERFORMANCE);
        assert_eq!(parse_guid(&guid_string(&g)), Some(g));
        let found = find_guid_in("Power Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (High performance)");
        assert_eq!(found, Some(g));
    }
}
