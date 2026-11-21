//! Settings saved as JSON in Luna's data folder (see `data_dir`).
//! Every struct uses `#[serde(default)]` so older or partial files still load.

use crate::color::ColorParams;
use crate::gamma::Tone;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Bind {
    /// Windows virtual-key code. Mouse buttons use the mouse VK codes
    /// (1 left, 2 right, 4 middle, 5 back, 6 forward).
    pub vk: u32,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Binds {
    pub zoom: Option<Bind>,
    pub nvg: Option<Bind>,
    pub tuning: Option<Bind>,
    pub color: Vec<Option<Bind>>,
    pub wheel: Option<Bind>,
    /// Switches the window in front between borderless and its normal frame.
    pub borderless: Option<Bind>,
    /// Shows or hides the FPS counter.
    pub fps: Option<Bind>,
    /// Turns Potato Mode on or off.
    pub potato: Option<Bind>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GammaPreset {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub tone: Tone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ColorPreset {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub color: ColorParams,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Nvg {
    /// "green" or "blue".
    pub mode: String,
    /// Monitor device name, empty = primary.
    pub monitor: String,
}

impl Default for Nvg {
    fn default() -> Self {
        Nvg { mode: "green".into(), monitor: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HoloSettings {
    /// "off", "greenTrueBlack" or "redBlackHolo".
    pub mode: String,
    /// 0 to 100 %.
    pub strength: f64,
}

impl Default for HoloSettings {
    fn default() -> Self {
        HoloSettings { mode: "off".into(), strength: 85.0 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Zoom {
    /// 1.5 to 8.
    pub factor: f64,
    pub monitor: String,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom { factor: 2.5, monitor: String::new() }
    }
}

/// Where a borderless window goes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Fit {
    /// "stay" (frame off, same size and place), "fill" (whole screen), "keep" (the game's own
    /// size, centred), a ratio such as "16:9", or "custom" (`width` x `height`), centred.
    pub mode: String,
    /// Monitor device name, empty = the screen the window is on.
    pub monitor: String,
    pub width: i32,
    pub height: i32,
}

impl Default for Fit {
    fn default() -> Self {
        Fit { mode: "stay".into(), monitor: String::new(), width: 1920, height: 1080 }
    }
}

/// A game made borderless by itself as soon as its window opens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BorderlessGame {
    /// File name of the program, like `game.exe`.
    pub exe: String,
    /// Window title when it was remembered, for the list.
    pub name: String,
    pub fit: Fit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Borderless {
    /// Used by the key and the Make borderless button.
    pub fit: Fit,
    pub games: Vec<BorderlessGame>,
}

/// A screen mode: size in pixels and refresh rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
    pub hz: i32,
}

/// What Luna changes while one game runs. Everything goes back when it closes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameProfile {
    /// File name of the program, like `game.exe`.
    pub exe: String,
    /// Shown in the list.
    pub name: String,
    pub enabled: bool,
    /// Monitor device name for the gamma preset and the screen mode, empty = the selected screen.
    pub monitor: String,
    /// Gamma preset id, empty = leave the screen as it is.
    pub gamma: String,
    /// Colour preset id, empty = leave the colour as it is.
    pub color: String,
    /// "" (leave it), "off", "on", or "slot0" to "slot7" for a saved crosshair.
    pub crosshair: String,
    /// Screen mode while the game runs, none = leave it.
    pub mode: Option<DisplayMode>,
    /// Potato Mode while the game runs.
    pub potato: bool,
}

impl Default for GameProfile {
    fn default() -> Self {
        GameProfile {
            exe: String::new(),
            name: String::new(),
            enabled: true,
            monitor: String::new(),
            gamma: String::new(),
            color: String::new(),
            crosshair: String::new(),
            mode: None,
            potato: false,
        }
    }
}

/// The FPS counter in a corner of a screen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FpsSettings {
    pub enabled: bool,
    /// "topLeft", "topRight", "bottomLeft" or "bottomRight".
    pub corner: String,
    /// Monitor device name, empty = primary.
    pub monitor: String,
}

impl Default for FpsSettings {
    fn default() -> Self {
        FpsSettings { enabled: false, corner: "topLeft".into(), monitor: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Crosshair {
    pub design: String,
    pub color: String,
    pub ring: f64,
    pub length: f64,
    pub gap: f64,
    pub thickness: f64,
    pub dot: f64,
    pub outline: f64,
    pub opacity: f64,
}

impl Default for Crosshair {
    fn default() -> Self {
        Crosshair {
            design: "classic".into(),
            color: "#3dff8b".into(),
            ring: 10.0,
            length: 8.0,
            gap: 4.0,
            thickness: 2.0,
            dot: 2.0,
            outline: 1.0,
            opacity: 100.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CrosshairSettings {
    pub enabled: bool,
    pub monitor: String,
    /// Global Ctrl+Alt+Shift+F11 toggle. Off by default.
    pub hotkey: bool,
    pub current: Crosshair,
    pub slots: Vec<Option<Crosshair>>,
    pub slot_names: Vec<String>,
}

impl Default for CrosshairSettings {
    fn default() -> Self {
        CrosshairSettings {
            enabled: false,
            monitor: String::new(),
            hotkey: false,
            current: Crosshair::default(),
            slots: vec![None; 8],
            slot_names: vec![String::new(); 8],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    /// "sky" (crescent moon), "nebula" or "custom".
    pub background: String,
    pub accent: String,
    /// 0 to 100.
    pub effect: f64,
    pub font: String,
    pub radius: String,
    /// Whole look: "nebula" (default), "observatoire", "calibration", "terrain" or "studio".
    pub skin: String,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: "sky".into(),
            accent: "#a86dff".into(),
            effect: 60.0,
            font: "hanken".into(),
            radius: "soft".into(),
            skin: "nebula".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppPrefs {
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    /// Checks GitHub for a newer signed version and installs it.
    pub auto_update: bool,
}

impl Default for AppPrefs {
    fn default() -> Self {
        AppPrefs { start_with_windows: false, start_minimized: false, close_to_tray: true, auto_update: true }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub tuning_on: bool,
    pub selected_monitor: String,
    /// Tone per monitor device name.
    pub tones: HashMap<String, Tone>,
    /// Colour applies to every screen.
    pub color: ColorParams,
    /// Monitor -> gamma preset id currently applied (for labels).
    pub active_gamma: HashMap<String, String>,
    pub active_color: String,
    /// "general" or "rust".
    pub profile: String,
    pub nvg: Nvg,
    pub holo: HoloSettings,
    pub zoom: Zoom,
    pub borderless: Borderless,
    pub binds: Binds,
    /// Four preset ids for the colour keys.
    pub color_keys: Vec<String>,
    /// Preset ids shown in the look wheel.
    pub wheel_presets: Vec<String>,
    pub gamma_presets: Vec<GammaPreset>,
    pub color_presets: Vec<ColorPreset>,
    pub presets_seeded: bool,
    pub crosshair: CrosshairSettings,
    /// Looks, screen mode and Potato Mode per game.
    pub games: Vec<GameProfile>,
    pub fps: FpsSettings,
    pub theme: Theme,
    pub app: AppPrefs,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            tuning_on: true,
            selected_monitor: String::new(),
            tones: HashMap::new(),
            color: ColorParams::default(),
            active_gamma: HashMap::new(),
            active_color: String::new(),
            profile: "general".into(),
            nvg: Nvg::default(),
            holo: HoloSettings::default(),
            zoom: Zoom::default(),
            borderless: Borderless::default(),
            binds: Binds {
                wheel: Some(Bind { vk: 0x76, label: "F7".into() }),
                color: vec![None; 4],
                ..Binds::default()
            },
            color_keys: vec![String::new(); 4],
            wheel_presets: Vec::new(),
            gamma_presets: Vec::new(),
            color_presets: Vec::new(),
            presets_seeded: false,
            crosshair: CrosshairSettings::default(),
            games: Vec::new(),
            fps: FpsSettings::default(),
            theme: Theme::default(),
            app: AppPrefs::default(),
        }
    }
}

/// `%APPDATA%\Luna` on Windows, `~/Library/Application Support/Luna` on macOS,
/// `$XDG_CONFIG_HOME/luna` (or `~/.config/luna`) on Linux.
pub fn data_dir() -> PathBuf {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        return var("APPDATA").unwrap_or_else(std::env::temp_dir).join("Luna");
    }
    let home = var("HOME").unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        return home.join("Library").join("Application Support").join("Luna");
    }
    var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config")).join("luna")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn load() -> Settings {
    match std::fs::read_to_string(settings_path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
            // Keep the broken file for the person to inspect, start fresh.
            let _ = std::fs::rename(settings_path(), data_dir().join("settings.broken.json"));
            Settings::default()
        }),
        Err(_) => Settings::default(),
    }
}

/// Writes atomically: a temp file first, then a rename.
pub fn save(s: &Settings) -> std::io::Result<()> {
    write_json_atomic(&settings_path(), s)
}

pub fn write_json_atomic<T: Serialize>(path: &std::path::Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_through_json() {
        let s = Settings::default();
        let text = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn partial_json_fills_defaults() {
        let s: Settings = serde_json::from_str(r##"{"tuningOn": false, "theme": {"accent": "#ff0000"}}"##).unwrap();
        assert!(!s.tuning_on);
        assert_eq!(s.theme.accent, "#ff0000");
        assert_eq!(s.theme.font, "hanken");
        assert_eq!(s.theme.skin, "nebula");
        assert_eq!(s.binds.wheel.as_ref().map(|b| b.vk), Some(0x76));
        assert_eq!(s.borderless.fit.mode, "stay");
        assert!(s.borderless.games.is_empty());
        assert!(s.binds.borderless.is_none());
    }

    #[test]
    fn skin_survives_a_round_trip() {
        let s: Settings = serde_json::from_str(r##"{"theme": {"skin": "terrain", "accent": "#9be37a"}}"##).unwrap();
        assert_eq!(s.theme.skin, "terrain");
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.theme.skin, "terrain");
        assert_eq!(back.theme.accent, "#9be37a");
    }

    #[test]
    fn game_profiles_fill_their_defaults() {
        let s: Settings =
            serde_json::from_str(r##"{"games": [{"exe": "ashfall.exe", "mode": {"width": 1920, "height": 1080, "hz": 144}}]}"##)
                .unwrap();
        let g = &s.games[0];
        assert!(g.enabled);
        assert!(!g.potato);
        assert!(g.gamma.is_empty() && g.crosshair.is_empty());
        assert_eq!(g.mode, Some(DisplayMode { width: 1920, height: 1080, hz: 144 }));
        assert!(!s.fps.enabled);
        assert_eq!(s.fps.corner, "topLeft");
        assert!(s.binds.fps.is_none() && s.binds.potato.is_none());
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.games, s.games);
    }

    #[test]
    fn auto_update_is_on_for_old_settings_files() {
        // Files written before 0.3.0 have no autoUpdate key.
        let s: Settings = serde_json::from_str(r##"{"app": {"closeToTray": false}}"##).unwrap();
        assert!(s.app.auto_update);
        assert!(!s.app.close_to_tray);
        let off: Settings = serde_json::from_str(r##"{"app": {"autoUpdate": false}}"##).unwrap();
        assert!(!off.app.auto_update);
    }
}
