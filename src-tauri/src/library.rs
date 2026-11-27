//! Games installed on the PC, so a profile can be added without starting the game first.
//! The stores write down where each game is; which program in that folder is the game
//! itself is worked out here.

use crate::plat;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub name: String,
    /// File name of the program Luna watches for, like `game.exe`.
    pub exe: String,
    /// "Steam", "Epic" or "GOG".
    pub store: String,
}

/// Programs that ship next to a game and are never the game.
const HELPERS: [&str; 24] = [
    "unins",
    "crash",
    "redist",
    "setup",
    "install",
    "anticheat",
    "battleye",
    "beservice",
    "dxsetup",
    "dotnet",
    "prereq",
    "cefprocess",
    "subprocess",
    "helper",
    "report",
    "updater",
    "handler",
    "start_protected_game",
    "bootstrap",
    "diagnostic",
    "webview",
    "overlay",
    "console",
    "editor",
];

fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The game among the programs of its folder, given as (path, size in bytes): the one the
/// engine names "shipping", else the one the store starts, else the largest. Launchers
/// only count when nothing else is left.
pub fn pick_exe(programs: &[(String, u64)], hint: Option<&str>) -> Option<String> {
    let named = |p: &&(String, u64), words: &[&str]| {
        let name = file_name(&p.0).to_lowercase();
        words.iter().any(|w| name.contains(w))
    };
    let games: Vec<&(String, u64)> = programs.iter().filter(|p| !named(p, &HELPERS)).collect();
    let real: Vec<&(String, u64)> = games.iter().copied().filter(|p| !named(p, &["launcher"])).collect();
    let pool = if real.is_empty() { games } else { real };
    let largest = |list: &[&(String, u64)]| list.iter().max_by_key(|p| p.1).map(|p| file_name(&p.0).to_string());
    let shipping: Vec<&(String, u64)> = pool.iter().copied().filter(|p| named(p, &["shipping"])).collect();
    if !shipping.is_empty() {
        return largest(&shipping);
    }
    if let Some(hint) = hint.map(file_name).filter(|h| !h.is_empty()) {
        if let Some(p) = pool.iter().find(|p| file_name(&p.0).eq_ignore_ascii_case(hint)) {
            return Some(file_name(&p.0).to_string());
        }
    }
    largest(&pool)
}

/// Every value written as `"key"  "value"` in a Valve text file, with `\\` read as `\`.
pub fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.trim().split('"');
        // "" key "" gap "" value ""
        let (Some(_), Some(k), Some(_), Some(v)) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if k.eq_ignore_ascii_case(key) {
            out.push(v.replace("\\\\", "\\"));
        }
    }
    out
}

/// Installed games from every store Luna reads, by name. One entry per program.
pub fn installed() -> Vec<InstalledGame> {
    let mut list = plat::library::installed();
    list.sort_by_key(|g| g.name.to_lowercase());
    let mut seen = std::collections::HashSet::new();
    list.retain(|g| !g.name.trim().is_empty() && seen.insert(g.exe.to_lowercase()));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[(&str, u64)]) -> Vec<(String, u64)> {
        items.iter().map(|(n, s)| (n.to_string(), *s)).collect()
    }

    #[test]
    fn the_shipping_program_wins_over_the_stub_the_store_starts() {
        let programs = list(&[
            ("Ashfall.exe", 300_000),
            ("Ashfall\\Binaries\\Win64\\Ashfall-Win64-Shipping.exe", 90_000_000),
            ("Engine\\Extras\\Redist\\UEPrereqSetup_x64.exe", 40_000_000),
            ("Ashfall\\Binaries\\Win64\\CrashReportClient.exe", 20_000_000),
        ]);
        assert_eq!(pick_exe(&programs, Some("Ashfall.exe")).as_deref(), Some("Ashfall-Win64-Shipping.exe"));
    }

    #[test]
    fn helpers_are_never_the_game() {
        let programs = list(&[("Game.exe", 650_000), ("UnityCrashHandler64.exe", 1_100_000), ("unins000.exe", 3_000_000)]);
        assert_eq!(pick_exe(&programs, None).as_deref(), Some("Game.exe"));
    }

    #[test]
    fn the_store_hint_beats_size() {
        let programs = list(&[("bin/tool.exe", 50_000_000), ("bin/game.exe", 9_000_000)]);
        assert_eq!(pick_exe(&programs, Some("bin/Game.exe")).as_deref(), Some("game.exe"));
        assert_eq!(pick_exe(&programs, None).as_deref(), Some("tool.exe"));
    }

    #[test]
    fn a_launcher_is_kept_when_it_is_all_there_is() {
        let programs = list(&[("GameLauncher.exe", 5_000_000), ("vc_redist.x64.exe", 25_000_000)]);
        assert_eq!(pick_exe(&programs, None).as_deref(), Some("GameLauncher.exe"));
        assert_eq!(pick_exe(&list(&[("vc_redist.x64.exe", 1)]), None), None);
    }

    #[test]
    fn reads_valve_text() {
        let text = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\Games\"\n\t}\n}";
        assert_eq!(vdf_values(text, "path"), vec!["C:\\Program Files (x86)\\Steam", "D:\\Games"]);
        assert_eq!(vdf_values("\t\"name\"\t\t\"Ashfall Frontier\"", "name"), vec!["Ashfall Frontier"]);
    }
}

#[cfg(test)]
mod live {
    /// Lists the games of this PC: `cargo test lists_this_pc -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn lists_this_pc() {
        let t = std::time::Instant::now();
        for g in super::installed() {
            println!("{:6} {:40} {}", g.store, g.name, g.exe);
        }
        println!("{:?}", t.elapsed());
    }
}
