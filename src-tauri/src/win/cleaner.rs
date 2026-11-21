//! The Clean page on Windows: caches, temporary files, logs and crash reports that programs make
//! again by themselves. Documents, game saves, passwords, history and cookies are never on the list.

use crate::sweep::{self, CleanItem, CleanReport, CleanResult, Spot, DAY};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::Shell::{
    IsUserAnAdmin, SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    SHQUERYRBINFO,
};

const RECYCLE: &str = "recycleBin";

struct Target {
    id: &'static str,
    group: &'static str,
    title: &'static str,
    desc: &'static str,
    suggested: bool,
    admin: bool,
    /// The program that writes these files, and the names its processes run under.
    app: Option<(&'static str, &'static [&'static str])>,
    spots: Vec<Spot>,
}

fn env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// `base\part` for every base that exists.
fn under(base: &Option<PathBuf>, part: &str) -> Option<PathBuf> {
    base.as_ref().map(|b| b.join(part))
}

fn all(dirs: impl IntoIterator<Item = Option<PathBuf>>) -> Vec<Spot> {
    dirs.into_iter().flatten().map(Spot::all).collect()
}

fn children(dir: &Path, keep: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
        .filter(|e| keep(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect()
}

/// Cache folders a Chromium-based program keeps in one profile folder.
fn chromium_profile(dir: &Path) -> Vec<Spot> {
    const CACHES: [&str; 8] =
        ["Cache", "Code Cache", "GPUCache", "ShaderCache", "GrShaderCache", "GraphiteDawnCache", "DawnGraphiteCache", "DawnWebGPUCache"];
    CACHES.iter().map(|c| dir.join(c)).filter(|p| p.is_dir()).map(Spot::all).collect()
}

/// A Chromium "User Data" folder: its own caches and those of every profile in it.
fn chromium(user_data: Option<PathBuf>) -> Vec<Spot> {
    let Some(root) = user_data.filter(|p| p.is_dir()) else { return Vec::new() };
    let mut spots = chromium_profile(&root);
    for profile in children(&root, |_| true) {
        spots.extend(chromium_profile(&profile));
    }
    spots
}

fn steam_dir() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    let key = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam").ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path.replace('/', "\\")))
}

fn targets() -> Vec<Target> {
    let local = env("LOCALAPPDATA");
    let roaming = env("APPDATA");
    let windir = env("WINDIR").or_else(|| env("SystemRoot"));
    let data = env("ProgramData");
    let home = env("USERPROFILE");
    let steam = steam_dir();
    let l = |p: &str| under(&local, p);
    let r = |p: &str| under(&roaming, p);
    let w = |p: &str| under(&windir, p);

    let mut list = vec![
        Target {
            id: "temp",
            group: "system",
            title: "Temporary files",
            desc: "Files programs leave in your temp folder. Only those older than a day go.",
            suggested: true,
            admin: false,
            app: None,
            spots: vec![Spot::older(std::env::temp_dir(), DAY)],
        },
        Target {
            id: "windowsTemp",
            group: "system",
            title: "Windows temporary files",
            desc: "The temp folder Windows and installers use. Only files older than a day go.",
            suggested: true,
            admin: true,
            app: None,
            spots: w("Temp").into_iter().map(|d| Spot::older(d, DAY)).collect(),
        },
        Target {
            id: "crashReports",
            group: "system",
            title: "Crash dumps and error reports",
            desc: "Copies of memory and reports kept after a program or game crashed.",
            suggested: true,
            admin: false,
            app: None,
            spots: all([
                l("CrashDumps"),
                l(r"Microsoft\Windows\WER\ReportArchive"),
                l(r"Microsoft\Windows\WER\ReportQueue"),
                l(r"Microsoft\Windows\WER\Temp"),
            ]),
        },
        Target {
            id: "systemCrashReports",
            group: "system",
            title: "Windows crash dumps",
            desc: "Memory dumps and reports Windows keeps after a blue screen or a driver crash.",
            suggested: true,
            admin: true,
            app: None,
            spots: {
                let mut s = all([
                    w("Minidump"),
                    w("LiveKernelReports"),
                    under(&data, r"Microsoft\Windows\WER\ReportArchive"),
                    under(&data, r"Microsoft\Windows\WER\ReportQueue"),
                    under(&data, r"Microsoft\Windows\WER\Temp"),
                ]);
                if let Some(d) = w("") {
                    s.push(Spot::files(d, |n| n.eq_ignore_ascii_case("MEMORY.DMP")));
                }
                s
            },
        },
        Target {
            id: "updateCache",
            group: "system",
            title: "Windows Update downloads",
            desc: "Update files already installed. Windows downloads again whatever it still needs.",
            suggested: true,
            admin: true,
            app: None,
            spots: all([
                w(r"SoftwareDistribution\Download"),
                w(r"ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache"),
            ]),
        },
        Target {
            id: "windowsLogs",
            group: "system",
            title: "Windows logs",
            desc: "Setup and maintenance logs older than a week.",
            suggested: true,
            admin: true,
            app: None,
            spots: w("Logs").into_iter().map(|d| Spot::older(d, 7 * DAY)).collect(),
        },
        Target {
            id: "thumbnails",
            group: "system",
            title: "Thumbnail cache",
            desc: "Small previews of pictures and videos. Windows draws them again, folders open a bit slower at first.",
            suggested: false,
            admin: false,
            app: None,
            spots: l(r"Microsoft\Windows\Explorer")
                .into_iter()
                .map(|d| Spot::files(d, |n| n.starts_with("thumbcache_") || n.starts_with("iconcache_")))
                .collect(),
        },
    ];

    let browsers: [(&str, &str, &[&str], Vec<Spot>); 7] = [
        ("chrome", "Chrome", &["chrome.exe"], chromium(l(r"Google\Chrome\User Data"))),
        ("edge", "Edge", &["msedge.exe"], chromium(l(r"Microsoft\Edge\User Data"))),
        ("brave", "Brave", &["brave.exe"], chromium(l(r"BraveSoftware\Brave-Browser\User Data"))),
        ("vivaldi", "Vivaldi", &["vivaldi.exe"], chromium(l(r"Vivaldi\User Data"))),
        ("opera", "Opera", &["opera.exe"], {
            let mut s = Vec::new();
            for name in ["Opera Stable", "Opera GX Stable"] {
                for base in [l(&format!(r"Opera Software\{name}")), r(&format!(r"Opera Software\{name}"))].into_iter().flatten() {
                    s.extend(chromium_profile(&base));
                }
            }
            s
        }),
        ("firefox", "Firefox", &["firefox.exe"], {
            let profiles = l(r"Mozilla\Firefox\Profiles").map(|p| children(&p, |_| true)).unwrap_or_default();
            profiles.iter().flat_map(|p| [p.join("cache2"), p.join("startupCache")]).filter(|p| p.is_dir()).map(Spot::all).collect()
        }),
        ("arc", "Arc", &["Arc.exe"], {
            let pkgs = l("Packages").map(|p| children(&p, |n| n.starts_with("TheBrowserCompany.Arc_"))).unwrap_or_default();
            pkgs.iter().flat_map(|p| chromium(Some(p.join(r"LocalCache\Local\Arc\User Data")))).collect()
        }),
    ];
    for (id, name, procs, spots) in browsers {
        list.push(Target {
            id,
            group: "browsers",
            title: name,
            desc: "Cached pages and pictures. History, passwords and cookies stay.",
            suggested: true,
            admin: false,
            app: Some((name, procs)),
            spots,
        });
    }

    let mut epic = l(r"EpicGamesLauncher\Saved").map(|p| children(&p, |n| n.starts_with("webcache"))).unwrap_or_default();
    epic.extend(l(r"EpicGamesLauncher\Saved\Logs"));
    list.extend([
        Target {
            id: "discord",
            group: "apps",
            title: "Discord",
            desc: "Cached pictures, emojis and videos from your chats.",
            suggested: true,
            admin: false,
            app: Some(("Discord", &["Discord.exe", "DiscordPTB.exe", "DiscordCanary.exe"])),
            spots: ["discord", "discordptb", "discordcanary"].iter().filter_map(|d| r(d)).flat_map(|d| chromium_profile(&d)).collect(),
        },
        Target {
            id: "steam",
            group: "apps",
            title: "Steam",
            desc: "Store and library page cache, old logs and crash dumps. Games and saves stay.",
            suggested: true,
            admin: false,
            app: Some(("Steam", &["steam.exe", "steamwebhelper.exe"])),
            spots: {
                let mut s = all([l(r"Steam\htmlcache")]);
                if let Some(st) = &steam {
                    s.extend(all([Some(st.join("dumps")), Some(st.join(r"appcache\httpcache"))]));
                    s.push(Spot::older(st.join("logs"), DAY));
                }
                s
            },
        },
        Target {
            id: "epic",
            group: "apps",
            title: "Epic Games Launcher",
            desc: "Store page cache and old logs. Games stay.",
            suggested: true,
            admin: false,
            app: Some(("Epic Games Launcher", &["EpicGamesLauncher.exe"])),
            spots: epic.into_iter().map(Spot::all).collect(),
        },
        Target {
            id: "battlenet",
            group: "apps",
            title: "Battle.net",
            desc: "Launcher page cache. Games stay.",
            suggested: true,
            admin: false,
            app: Some(("Battle.net", &["Battle.net.exe"])),
            spots: all([l(r"Battle.net\Cache"), l(r"Battle.net\BrowserCaches"), l(r"Blizzard Entertainment\Battle.net\Cache")]),
        },
        Target {
            id: "spotify",
            group: "apps",
            title: "Spotify",
            desc: "Songs kept to play them faster. Downloads for offline listening stay.",
            suggested: true,
            admin: false,
            app: Some(("Spotify", &["Spotify.exe"])),
            spots: {
                let mut s = all([l(r"Spotify\Data"), l(r"Spotify\Browser\Cache")]);
                let store = l("Packages").map(|p| children(&p, |n| n.starts_with("SpotifyAB.SpotifyMusic_"))).unwrap_or_default();
                s.extend(store.iter().map(|p| p.join(r"LocalCache\Spotify\Data")).filter(|p| p.is_dir()).map(Spot::all));
                s
            },
        },
        Target {
            id: "chatApps",
            group: "apps",
            title: "Teams and Slack",
            desc: "Cached pictures and pages from your chats.",
            suggested: true,
            admin: false,
            app: Some(("Teams or Slack", &["Teams.exe", "ms-teams.exe", "slack.exe"])),
            spots: [r(r"Microsoft\Teams"), r("Slack")].into_iter().flatten().flat_map(|d| chromium_profile(&d)).collect(),
        },
        Target {
            id: "shaders",
            group: "apps",
            title: "Graphics card shader cache",
            desc: "Shaders games build on your graphics card. They are rebuilt on the next launch, which can stutter for a few minutes. Worth it after a driver update.",
            suggested: false,
            admin: false,
            app: None,
            spots: all([
                l(r"NVIDIA\DXCache"),
                l(r"NVIDIA\GLCache"),
                l(r"NVIDIA\OptixCache"),
                l(r"NVIDIA Corporation\NV_Cache"),
                l(r"AMD\DxCache"),
                l(r"AMD\DxcCache"),
                l(r"AMD\Dx9Cache"),
                l(r"AMD\VkCache"),
                l(r"AMD\GLCache"),
                l("D3DSCache"),
                l(r"Intel\ShaderCache"),
            ]),
        },
        Target {
            id: "devCaches",
            group: "dev",
            title: "Package caches",
            desc: "Downloads kept by npm, Yarn, pip, NuGet, Cargo and Go. Projects keep working, the next install downloads again.",
            suggested: false,
            admin: false,
            app: None,
            spots: all([
                l("npm-cache"),
                l(r"Yarn\Cache"),
                l(r"pip\cache"),
                l(r"NuGet\v3-cache"),
                l("go-build"),
                under(&home, r".cargo\registry\cache"),
            ]),
        },
    ]);
    list
}

fn admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

/// Lower-case names of every running program.
fn running() -> HashSet<String> {
    let mut names = HashSet::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return names };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut entry).is_ok();
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            names.insert(String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase());
            ok = Process32NextW(snap, &mut entry).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    names
}

fn recycle_bin() -> (u64, u64) {
    let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
    match unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) } {
        Ok(()) => (info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64),
        Err(_) => (0, 0),
    }
}

/// Everything the page can clean, with its size today. Empty items are left out.
pub fn scan() -> CleanReport {
    let admin = admin();
    let open = running();
    let targets = targets();
    // Each kind is measured on its own thread: the temp folder alone can hold tens of thousands of files.
    let sizes: Vec<Option<(u64, u64)>> = std::thread::scope(|s| {
        let jobs: Vec<_> = targets
            .iter()
            .map(|t| s.spawn(move || t.spots.iter().any(Spot::exists).then(|| sweep::measure(&t.spots))))
            .collect();
        jobs.into_iter().map(|j| j.join().unwrap_or(None)).collect()
    });
    let mut items: Vec<CleanItem> = targets
        .into_iter()
        .zip(sizes)
        .filter_map(|(t, size)| {
            let (bytes, files) = size?;
            let locked = t.admin && !admin;
            if bytes == 0 && !locked {
                return None;
            }
            let open = t.app.and_then(|(name, procs)| procs.iter().any(|p| open.contains(&p.to_lowercase())).then(|| name.to_string()));
            Some(CleanItem {
                id: t.id.into(),
                group: t.group.into(),
                title: t.title.into(),
                desc: t.desc.into(),
                bytes,
                files,
                suggested: t.suggested,
                needs_admin: locked,
                open,
            })
        })
        .collect();

    let (bytes, files) = recycle_bin();
    if files > 0 {
        items.push(CleanItem {
            id: RECYCLE.into(),
            group: "system".into(),
            title: "Recycle Bin".into(),
            desc: "Everything you deleted. It cannot be brought back after this.".into(),
            bytes,
            files,
            suggested: false,
            needs_admin: false,
            open: None,
        });
    }
    CleanReport { items, admin }
}

/// Cleans the items with these ids. What needs administrator is left alone when Luna is not.
pub fn run(ids: &[String]) -> CleanResult {
    let mut res = CleanResult::default();
    let admin = admin();
    let wanted = |id: &str| ids.iter().any(|i| i == id);
    for t in targets() {
        if wanted(t.id) && (admin || !t.admin) {
            sweep::clear(&t.spots, &mut res);
        }
    }
    if wanted(RECYCLE) {
        let (bytes, files) = recycle_bin();
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        if unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), flags) }.is_ok() {
            res.freed_bytes += bytes;
            res.files += files;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    /// Prints what the page would show on this PC. Run with `cargo test scan_here -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn scan_here() {
        let t = std::time::Instant::now();
        let r = super::scan();
        for i in &r.items {
            println!("{:<10} {:<34} {:>9.1} MB {:>7} files  admin:{} open:{:?}", i.group, i.title, i.bytes as f64 / 1e6, i.files, i.needs_admin, i.open);
        }
        println!("admin {} in {:?}", r.admin, t.elapsed());
    }
}
