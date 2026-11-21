//! Settings saved as JSON in %APPDATA%\Luna\settings.json.
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
    /// "sky" or "custom".
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
}

impl Default for AppPrefs {
    fn default() -> Self {
        AppPrefs { start_with_windows: false, start_minimized: false, close_to_tray: true }
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
    pub binds: Binds,
    /// Four preset ids for the colour keys.
    pub color_keys: Vec<String>,
    /// Preset ids shown in the look wheel.
    pub wheel_presets: Vec<String>,
    pub gamma_presets: Vec<GammaPreset>,
    pub color_presets: Vec<ColorPreset>,
    pub presets_seeded: bool,
    pub crosshair: CrosshairSettings,
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
            theme: Theme::default(),
            app: AppPrefs::default(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Luna")
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
    }

    #[test]
    fn skin_survives_a_round_trip() {
        let s: Settings = serde_json::from_str(r##"{"theme": {"skin": "terrain", "accent": "#9be37a"}}"##).unwrap();
        assert_eq!(s.theme.skin, "terrain");
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.theme.skin, "terrain");
        assert_eq!(back.theme.accent, "#9be37a");
    }
}
