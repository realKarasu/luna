//! Where Steam, Epic and GOG write down the games they installed. Only read, never changed.

use crate::library::{pick_exe, vdf_values, InstalledGame};
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

/// Folders deep enough for `Game\Binaries\Win64\Game-Win64-Shipping.exe`.
const MAX_DEPTH: usize = 4;
/// A stop for folders with a huge number of files.
const MAX_ENTRIES: usize = 20_000;

/// Every program in a game folder as (path, size).
fn programs(root: &Path) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut seen = 0usize;
    let mut todo: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return out;
            }
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                if depth < MAX_DEPTH {
                    todo.push((path, depth + 1));
                }
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((path.to_string_lossy().into_owned(), size));
            }
        }
    }
    out
}

fn steam_root() -> Option<PathBuf> {
    let user = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|k| k.get_value::<String, _>("SteamPath"));
    let machine = || {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
            .and_then(|k| k.get_value::<String, _>("InstallPath"))
    };
    let path = user.or_else(|_| machine()).ok()?;
    Some(PathBuf::from(path.replace('/', "\\")))
}

/// Steam tools that are not games.
fn steam_tool(appid: &str, name: &str) -> bool {
    let name = name.to_lowercase();
    appid == "228980" || appid == "250820" || name.starts_with("proton") || name.contains("steam linux runtime") || name.contains("steamworks")
}

fn steam(out: &mut Vec<InstalledGame>) {
    let Some(root) = steam_root() else { return };
    let mut libraries = vec![root.clone()];
    if let Ok(text) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for p in vdf_values(&text, "path") {
            let p = PathBuf::from(p);
            if !libraries.iter().any(|l| l.to_string_lossy().eq_ignore_ascii_case(&p.to_string_lossy())) {
                libraries.push(p);
            }
        }
    }
    for lib in libraries {
        let apps = lib.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&apps) else { continue };
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().to_lowercase();
            if !file.starts_with("appmanifest_") || !file.ends_with(".acf") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            let first = |key: &str| vdf_values(&text, key).into_iter().next().unwrap_or_default();
            let (appid, name, dir) = (first("appid"), first("name"), first("installdir"));
            if name.is_empty() || dir.is_empty() || steam_tool(&appid, &name) {
                continue;
            }
            if let Some(exe) = pick_exe(&programs(&apps.join("common").join(&dir)), None) {
                out.push(InstalledGame { name, exe, store: "Steam".into() });
            }
        }
    }
}

fn epic(out: &mut Vec<InstalledGame>) {
    let data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = Path::new(&data).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests");
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if !entry.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("item")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let Ok(item) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let text_of = |key: &str| item.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let flag = |key: &str| item.get(key).and_then(|v| v.as_bool());
        // Add-ons have their own entries; a game still downloading has no program yet.
        if flag("bIsApplication") == Some(false) || flag("bIsIncompleteInstall") == Some(true) {
            continue;
        }
        let (name, location, launch) = (text_of("DisplayName"), text_of("InstallLocation"), text_of("LaunchExecutable"));
        if name.is_empty() || location.is_empty() {
            continue;
        }
        if let Some(exe) = pick_exe(&programs(Path::new(&location)), Some(&launch)) {
            out.push(InstalledGame { name, exe, store: "Epic".into() });
        }
    }
}

fn gog(out: &mut Vec<InstalledGame>) {
    let Ok(games) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\GOG.com\Games") else {
        return;
    };
    for id in games.enum_keys().flatten() {
        let Ok(key) = games.open_subkey(&id) else { continue };
        let text_of = |name: &str| key.get_value::<String, _>(name).unwrap_or_default();
        // Add-ons name the game they belong to.
        if !text_of("dependsOn").is_empty() {
            continue;
        }
        let name = text_of("gameName");
        let exe = text_of("exeFile");
        let exe = if exe.is_empty() { text_of("exe") } else { exe };
        let exe = exe.rsplit(['\\', '/']).next().unwrap_or_default().to_string();
        if !name.is_empty() && exe.to_lowercase().ends_with(".exe") {
            out.push(InstalledGame { name, exe, store: "GOG".into() });
        }
    }
}

pub fn installed() -> Vec<InstalledGame> {
    let mut out = Vec::new();
    steam(&mut out);
    epic(&mut out);
    gog(&mut out);
    out
}
