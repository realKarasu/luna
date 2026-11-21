//! The engine owns the live state and pushes it to Windows: gamma ramps per
//! monitor, the full-screen colour matrix, driver vibrance (NVIDIA or AMD, per
//! monitor), Ultra Zoom and the overlay windows. It also reacts to the input
//! hooks and to display changes.
//!
//! Locking rule: code holding the engine lock never waits on the UI thread
//! (window calls are asynchronous, tray updates go through a channel).

use crate::color::{self, ColorParams, Holo};
use crate::gamma::{self, Ramp, Tone};
use crate::settings::{self, Settings};
use crate::win::display::{self, MonitorInfo};
use crate::win::hooks::{self, HookEvent};
use crate::vibrance::{self, Vendor, VibranceBackend};
use crate::win::adl::AdlLib;
use crate::win::nvapi::NvLib;
use crate::win::{gpu, mag, window};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, TryLockError};
use tauri::{AppHandle, Emitter};

pub const CROSSHAIR_SIZE: i32 = 256;
pub const WHEEL_SIZE: i32 = 560;

static ENGINE: OnceLock<Mutex<Engine>> = OnceLock::new();

pub fn engine() -> MutexGuard<'static, Engine> {
    ENGINE.get().expect("engine started").lock().unwrap_or_else(|e| e.into_inner())
}

pub fn engine_ready() -> bool {
    ENGINE.get().is_some()
}

/// Messages for the tray (handled outside the engine lock).
pub enum UiMsg {
    Tray { tuning: bool, crosshair: bool },
}

/// Messages handled by the engine thread.
pub enum EngineMsg {
    Hook(HookEvent),
    ToggleTuning,
    ToggleCrosshair,
    Reset,
    Tick,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelItem {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WheelState {
    visible: bool,
    items: Vec<WheelItem>,
    selected: i32,
}

struct Wheel {
    items: Vec<WheelItem>,
    dx: f64,
    dy: f64,
    selected: i32,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct RestoreFile {
    ramps: HashMap<String, Vec<Vec<u16>>>,
    /// Each monitor's driver vibrance level before Luna changed it.
    driver_vibrance: HashMap<String, i32>,
}

/// One monitor's driver vibrance and the level last written to it.
struct VibSlot {
    backend: Box<dyn VibranceBackend>,
    last: Option<i32>,
}

impl VibSlot {
    fn write(&mut self, level: i32) {
        if self.last != Some(level) && self.backend.set(level) {
            self.last = Some(level);
        }
    }
}

fn restore_path() -> std::path::PathBuf {
    settings::data_dir().join("restore.json")
}

fn ramp_to_vec(r: &Ramp) -> Vec<Vec<u16>> {
    r.iter().map(|c| c.to_vec()).collect()
}

fn vec_to_ramp(v: &[Vec<u16>]) -> Option<Ramp> {
    if v.len() != 3 || v.iter().any(|c| c.len() != 256) {
        return None;
    }
    let mut r = [[0u16; 256]; 3];
    for ch in 0..3 {
        r[ch].copy_from_slice(&v[ch]);
    }
    Some(r)
}

/// Puts back the ramps saved at start. Used by the panic hook: no engine lock needed.
pub fn restore_from_file() {
    if let Ok(text) = std::fs::read_to_string(restore_path()) {
        if let Ok(f) = serde_json::from_str::<RestoreFile>(&text) {
            for (id, r) in &f.ramps {
                if let Some(r) = vec_to_ramp(r) {
                    display::set_ramp(id, &r);
                }
            }
        }
    }
}

/// Puts driver vibrance back from the panic hook. Skipped when the panicking
/// thread holds the engine; the next start then restores it from restore.json.
pub fn restore_vibrance_after_panic() {
    let Some(m) = ENGINE.get() else { return };
    let guard = match m.try_lock() {
        Ok(g) => Some(g),
        Err(TryLockError::Poisoned(p)) => Some(p.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    };
    if let Some(mut e) = guard {
        e.restore_vibrance();
    }
}

fn ramp_distance(a: &Ramp, b: &Ramp) -> u16 {
    let mut m = 0;
    for ch in 0..3 {
        for i in 0..256 {
            m = m.max(a[ch][i].abs_diff(b[ch][i]));
        }
    }
    m
}

pub fn nvg_tone(mode: &str) -> Tone {
    match mode {
        "blue" => Tone { gamma: 3.0, brightness: 0.0, contrast: 100.0, red: 35.0, green: 70.0, blue: 100.0 },
        _ => Tone { gamma: 3.0, brightness: 0.0, contrast: 100.0, red: 35.0, green: 100.0, blue: 35.0 },
    }
}

pub struct Engine {
    pub settings: Settings,
    pub monitors: Vec<MonitorInfo>,
    originals: HashMap<String, Ramp>,
    applied: HashMap<String, Ramp>,
    pub gamma_strength: HashMap<String, f64>,
    nv_lib: Option<NvLib>,
    adl_lib: Option<Arc<AdlLib>>,
    /// Driver vibrance per monitor id; monitors without one use the matrix.
    vib: HashMap<String, VibSlot>,
    /// First driver level seen per monitor id: what quit, reset and crash recovery put back.
    vib_originals: HashMap<String, i32>,
    nvg_held: bool,
    zoom_held: bool,
    wheel: Option<Wheel>,
    pub recording: bool,
    app: AppHandle,
    ui: Sender<UiMsg>,
    crosshair_hwnd: isize,
    wheel_hwnd: isize,
    dirty: bool,
    shut_down: bool,
    ticks: u64,
}

impl Engine {
    /// Starts the engine: crash recovery, saving originals, NVAPI, magnifier.
    pub fn start(app: AppHandle, ui: Sender<UiMsg>, settings: Settings) {
        let monitors = display::enumerate();

        // If the last run did not exit cleanly, its saved originals are still on disk.
        let previous: Option<RestoreFile> =
            std::fs::read_to_string(restore_path()).ok().and_then(|t| serde_json::from_str(&t).ok());
        if let Some(prev) = &previous {
            for (id, r) in &prev.ramps {
                if let Some(r) = vec_to_ramp(r) {
                    display::set_ramp(id, &r);
                }
            }
        }

        let mut originals = HashMap::new();
        for m in &monitors {
            let r = display::get_ramp(&m.id).unwrap_or_else(gamma::identity_ramp);
            originals.insert(m.id.clone(), r);
        }

        mag::start();

        let mut e = Engine {
            settings,
            monitors,
            originals,
            applied: HashMap::new(),
            gamma_strength: HashMap::new(),
            nv_lib: None,
            adl_lib: None,
            vib: HashMap::new(),
            vib_originals: HashMap::new(),
            nvg_held: false,
            zoom_held: false,
            wheel: None,
            recording: false,
            app,
            ui,
            crosshair_hwnd: 0,
            wheel_hwnd: 0,
            dirty: false,
            shut_down: false,
            ticks: 0,
        };
        e.detect_vibrance(false);
        // After a crash, the drivers still hold Luna's levels: put back the saved
        // originals and keep treating them as the originals.
        if let Some(prev) = &previous {
            for (id, level) in &prev.driver_vibrance {
                if let Some(slot) = e.vib.get_mut(id) {
                    slot.backend.set_original(*level);
                    let o = slot.backend.original();
                    slot.write(o);
                    e.vib_originals.insert(id.clone(), o);
                }
            }
        }
        e.write_restore_file();
        e.fix_selection();
        e.apply_all();
        let _ = ENGINE.set(Mutex::new(e));
    }

    fn write_restore_file(&self) {
        let f = RestoreFile {
            ramps: self.originals.iter().map(|(k, v)| (k.clone(), ramp_to_vec(v))).collect(),
            driver_vibrance: self.vib_originals.clone(),
        };
        let _ = settings::write_json_atomic(&restore_path(), &f);
    }

    /// Finds the graphics card behind each monitor (DXGI) and picks its vibrance
    /// backend: NVAPI on NVIDIA, ADL on AMD, the colour matrix otherwise or when
    /// a driver library is missing or refuses. `reload` starts ADL afresh, as
    /// its adapter list can change when screens come and go.
    fn detect_vibrance(&mut self, reload: bool) {
        self.vib.clear();
        if reload {
            self.adl_lib = None;
        }
        let gpus = gpu::detect();
        for m in self.monitors.iter_mut() {
            match gpus.get(&gpu::key(&m.id)) {
                Some(g) => {
                    m.gpu = g.name.clone();
                    m.vendor = g.vendor;
                }
                None => {
                    m.gpu = String::new();
                    m.vendor = Vendor::Generic;
                }
            }
            m.vibrance = display::MATRIX_VIBRANCE.to_string();
        }
        // When DXGI tells nothing, a driver that names the monitor is trusted.
        let wants = |m: &MonitorInfo, v: Vendor| gpus.is_empty() || m.vendor == v;
        let any = |v: Vendor| gpus.is_empty() || self.monitors.iter().any(|m| m.vendor == v);
        let mut found: HashMap<String, Box<dyn VibranceBackend>> = HashMap::new();

        if any(Vendor::Nvidia) {
            if self.nv_lib.is_none() {
                self.nv_lib = NvLib::load();
            }
            if let Some(nv) = &self.nv_lib {
                let mut unnamed = Vec::new();
                for (name, d) in nv.displays() {
                    let target = name.and_then(|n| {
                        self.monitors.iter().find(|m| m.id.eq_ignore_ascii_case(&n) && wants(m, Vendor::Nvidia))
                    });
                    match target {
                        Some(m) if !found.contains_key(&m.id) => {
                            found.insert(m.id.clone(), Box::new(d));
                        }
                        Some(_) => {}
                        None => unnamed.push(d),
                    }
                }
                // Older drivers do not name their displays: hand them out in order.
                let mut rest = unnamed.into_iter();
                for m in self.monitors.iter().filter(|m| m.vendor == Vendor::Nvidia) {
                    if found.contains_key(&m.id) {
                        continue;
                    }
                    match rest.next() {
                        Some(d) => {
                            found.insert(m.id.clone(), Box::new(d));
                        }
                        None => break,
                    }
                }
            }
        }

        if any(Vendor::Amd) {
            if self.adl_lib.is_none() {
                self.adl_lib = AdlLib::load();
            }
            if let Some(lib) = &self.adl_lib {
                for (device, d) in AdlLib::displays(lib) {
                    if let Some(m) =
                        self.monitors.iter().find(|m| m.id.eq_ignore_ascii_case(&device) && wants(m, Vendor::Amd))
                    {
                        found.entry(m.id.clone()).or_insert_with(|| Box::new(d));
                    }
                }
            }
        }

        for (id, mut backend) in found {
            match self.vib_originals.get(&id) {
                Some(o) => backend.set_original(*o),
                None => {
                    self.vib_originals.insert(id.clone(), backend.original());
                }
            }
            let last = backend.get();
            self.vib.insert(id, VibSlot { backend, last });
        }
        if self.vib.is_empty() {
            // Nothing uses AMD's context any more.
            self.adl_lib = None;
        }

        // A shared matrix: when one screen has no driver vibrance, the matrix
        // does the whole job on every screen and the drivers stay as they were.
        if self.vib_cover().is_some() {
            for m in self.monitors.iter_mut() {
                if let Some(slot) = self.vib.get(&m.id) {
                    m.vibrance = slot.backend.name().to_string();
                }
            }
        }
    }

    /// The vibrance range every monitor's driver can show, in Luna percent.
    fn vib_cover(&self) -> Option<(f64, f64)> {
        let covers: Vec<Option<(f64, f64)>> =
            self.monitors.iter().map(|m| self.vib.get(&m.id).map(|s| s.backend.range().covers())).collect();
        vibrance::common_cover(&covers)
    }

    /// Every driver back to the level it had before Luna.
    fn restore_vibrance(&mut self) {
        for slot in self.vib.values_mut() {
            let o = slot.backend.original();
            slot.write(o);
        }
    }

    /// The screen layout or a graphics card changed: read everything again.
    fn refresh_displays(&mut self, now: Vec<MonitorInfo>) {
        if self.shut_down {
            return;
        }
        for m in &now {
            if !self.originals.contains_key(&m.id) {
                let r = display::get_ramp(&m.id).unwrap_or_else(gamma::identity_ramp);
                self.originals.insert(m.id.clone(), r);
            }
        }
        self.monitors = now;
        // A mode change can reset the ramps, so write them all again.
        self.applied.clear();
        self.gamma_strength.retain(|id, _| self.monitors.iter().any(|m| &m.id == id));
        self.detect_vibrance(true);
        self.fix_selection();
        self.write_restore_file();
        self.apply_all();
        let _ = self.app.emit("monitors-changed", &self.monitors);
    }

    pub fn set_overlay_windows(&mut self, crosshair: isize, wheel: isize) {
        self.crosshair_hwnd = crosshair;
        self.wheel_hwnd = wheel;
        self.apply_crosshair();
    }

    /// At least one monitor gets its vibrance from the graphics driver.
    pub fn driver_vibrance(&self) -> bool {
        self.vib_cover().is_some()
    }

    fn primary(&self) -> Option<&MonitorInfo> {
        self.monitors.iter().find(|m| m.primary).or(self.monitors.first())
    }

    fn monitor_or_primary(&self, id: &str) -> Option<MonitorInfo> {
        self.monitors.iter().find(|m| m.id == id).or_else(|| self.primary()).cloned()
    }

    fn fix_selection(&mut self) {
        if !self.monitors.iter().any(|m| m.id == self.settings.selected_monitor) {
            if let Some(p) = self.primary() {
                self.settings.selected_monitor = p.id.clone();
            }
        }
    }

    // ---------------- applying ----------------

    fn target_tone(&self, m: &MonitorInfo) -> Option<Tone> {
        if self.nvg_held && self.settings.profile == "rust" {
            let nvg_mon = self.monitor_or_primary(&self.settings.nvg.monitor);
            if nvg_mon.map(|n| n.id == m.id).unwrap_or(false) {
                return Some(nvg_tone(&self.settings.nvg.mode));
            }
        }
        if !self.settings.tuning_on {
            return None;
        }
        self.settings.tones.get(&m.id).copied().filter(|t| !t.is_neutral())
    }

    fn apply_gamma(&mut self) {
        let monitors = self.monitors.clone();
        for m in &monitors {
            let original = *self.originals.get(&m.id).unwrap_or(&gamma::identity_ramp());
            match self.target_tone(m) {
                None => {
                    if self.applied.remove(&m.id).is_some() {
                        display::set_ramp(&m.id, &original);
                    }
                    self.gamma_strength.remove(&m.id);
                }
                Some(tone) => {
                    let ramp = gamma::build_ramp(&tone);
                    if self.applied.get(&m.id) == Some(&ramp) {
                        continue;
                    }
                    match display::set_ramp_best_effort(&m.id, &original, &ramp) {
                        Some(f) => {
                            self.gamma_strength.insert(m.id.clone(), f);
                        }
                        None => {
                            self.gamma_strength.insert(m.id.clone(), 0.0);
                        }
                    }
                    self.applied.insert(m.id.clone(), ramp);
                }
            }
        }
    }

    fn apply_color(&mut self) {
        let cover = self.vib_cover();
        let s = &self.settings;
        let (matrix, vib) = if s.tuning_on {
            let holo = if s.profile == "rust" { Holo::from_name(&s.holo.mode) } else { Holo::Off };
            let p: ColorParams = s.color.clamped();
            // The drivers show what they can; the matrix adds the rest.
            let factor = color::vibrance_matrix_factor(p.vibrance, cover);
            (color::color_matrix(&p, factor, holo, s.holo.strength / 100.0), Some(p.vibrance))
        } else {
            (color::identity(), None)
        };
        let driver = vib.map(|v| vibrance::driver_percent(v, cover));
        for slot in self.vib.values_mut() {
            let level = match driver {
                Some(p) if (p - 100.0).abs() > 0.5 => slot.backend.range().level_for(p),
                _ => slot.backend.original(),
            };
            slot.write(level);
        }
        mag::set_color(color::to_mag(&matrix));
    }

    fn apply_zoom(&self) {
        if self.zoom_held {
            if let Some(m) = self.monitor_or_primary(&self.settings.zoom.monitor) {
                let f = self.settings.zoom.factor.clamp(1.5, 8.0);
                let w = m.width as f64 / f;
                let h = m.height as f64 / f;
                let x = m.x + ((m.width as f64 - w) / 2.0).round() as i32;
                let y = m.y + ((m.height as f64 - h) / 2.0).round() as i32;
                mag::set_zoom(f as f32, x, y);
            }
        } else {
            mag::set_zoom(1.0, 0, 0);
        }
    }

    fn apply_crosshair(&self) {
        if self.crosshair_hwnd == 0 {
            return;
        }
        let c = &self.settings.crosshair;
        match self.monitor_or_primary(&c.monitor) {
            Some(m) if c.enabled => {
                let cx = m.x + m.width / 2;
                let cy = m.y + m.height / 2;
                let half = CROSSHAIR_SIZE / 2;
                window::place(self.crosshair_hwnd, cx - half, cy - half, CROSSHAIR_SIZE, CROSSHAIR_SIZE, true);
            }
            _ => window::place(self.crosshair_hwnd, 0, 0, 0, 0, false),
        }
        let _ = self.app.emit_to("crosshair", "crosshair", &c.current);
    }

    fn notify_tray(&self) {
        let _ = self.ui.send(UiMsg::Tray { tuning: self.settings.tuning_on, crosshair: self.settings.crosshair.enabled });
    }

    pub fn apply_all(&mut self) {
        if self.shut_down {
            return;
        }
        self.apply_gamma();
        self.apply_color();
        self.apply_zoom();
        self.apply_crosshair();
        self.notify_tray();
    }

    // ---------------- changes ----------------

    /// New settings from the main window.
    pub fn set_settings(&mut self, s: Settings) {
        self.settings = s;
        self.fix_selection();
        self.dirty = true;
        self.apply_all();
    }

    /// Changes made here (binds, tray) are sent back to the main window.
    fn changed_here(&mut self) {
        self.dirty = true;
        self.apply_all();
        let _ = self.app.emit("settings-changed", &self.settings);
    }

    pub fn toggle_tuning(&mut self) {
        self.settings.tuning_on = !self.settings.tuning_on;
        self.changed_here();
    }

    pub fn toggle_crosshair(&mut self) {
        self.settings.crosshair.enabled = !self.settings.crosshair.enabled;
        self.changed_here();
    }

    /// Neutral tone on every screen, neutral colour, originals back.
    pub fn reset(&mut self) {
        self.settings.tones.clear();
        self.settings.active_gamma.clear();
        self.settings.color = ColorParams::default();
        self.settings.active_color.clear();
        self.settings.holo.mode = "off".into();
        for m in self.monitors.clone() {
            if let Some(o) = self.originals.get(&m.id) {
                display::set_ramp(&m.id, o);
            }
        }
        self.applied.clear();
        self.gamma_strength.clear();
        self.restore_vibrance();
        self.changed_here();
    }

    /// Applies a colour or gamma preset by id (colour keys and the look wheel).
    pub fn apply_preset(&mut self, id: &str) {
        if let Some(p) = self.settings.color_presets.iter().find(|p| p.id == id).cloned() {
            self.settings.color = p.color;
            self.settings.active_color = p.id;
        } else if let Some(p) = self.settings.gamma_presets.iter().find(|p| p.id == id).cloned() {
            let mon = self.settings.selected_monitor.clone();
            self.settings.tones.insert(mon.clone(), p.tone);
            self.settings.active_gamma.insert(mon, p.id);
        } else {
            return;
        }
        self.settings.tuning_on = true;
        self.changed_here();
    }

    // ---------------- input ----------------

    pub fn handle(&mut self, msg: EngineMsg) {
        match msg {
            EngineMsg::Hook(e) => self.handle_hook(e),
            EngineMsg::ToggleTuning => self.toggle_tuning(),
            EngineMsg::ToggleCrosshair => self.toggle_crosshair(),
            EngineMsg::Reset => self.reset(),
            EngineMsg::Tick => self.tick(),
        }
    }

    fn bind_is(b: &Option<settings::Bind>, vk: u32) -> bool {
        matches!(b, Some(b) if b.vk != 0 && b.vk == vk)
    }

    fn handle_hook(&mut self, e: HookEvent) {
        match e {
            HookEvent::Down(vk) => {
                if self.recording {
                    // A left click while recording is a click in Luna's own window
                    // (Cancel, another box). Mouse 1 is offered as a button instead.
                    if vk == 1 {
                        return;
                    }
                    self.recording = false;
                    let payload = if vk == 0x1B {
                        None
                    } else {
                        Some(settings::Bind { vk, label: hooks::key_label(vk) })
                    };
                    let _ = self.app.emit("bind-recorded", payload);
                    return;
                }
                if vk == 0x7A
                    && self.settings.crosshair.hotkey
                    && hooks::is_down(0x11)
                    && hooks::is_down(0x12)
                    && hooks::is_down(0x10)
                {
                    self.toggle_crosshair();
                    return;
                }
                // Mouse binds do nothing while you click around in Luna itself.
                if vk <= 6 && window::luna_in_front() {
                    return;
                }
                let b = self.settings.binds.clone();
                if Self::bind_is(&b.zoom, vk) && !self.zoom_held {
                    self.zoom_held = true;
                    self.apply_zoom();
                }
                if Self::bind_is(&b.nvg, vk) && self.settings.profile == "rust" && !self.nvg_held {
                    self.nvg_held = true;
                    self.apply_gamma();
                }
                if Self::bind_is(&b.tuning, vk) {
                    self.toggle_tuning();
                }
                for (i, cb) in b.color.iter().enumerate() {
                    if Self::bind_is(cb, vk) {
                        if let Some(id) = self.settings.color_keys.get(i).cloned() {
                            if !id.is_empty() {
                                self.apply_preset(&id);
                            }
                        }
                    }
                }
                if Self::bind_is(&b.wheel, vk) && self.wheel.is_none() {
                    self.open_wheel();
                }
            }
            HookEvent::Up(vk) => {
                let b = &self.settings.binds;
                if Self::bind_is(&b.zoom, vk) && self.zoom_held {
                    self.zoom_held = false;
                    self.apply_zoom();
                }
                if Self::bind_is(&b.nvg, vk) && self.nvg_held {
                    self.nvg_held = false;
                    self.apply_gamma();
                }
                if Self::bind_is(&self.settings.binds.wheel, vk) && self.wheel.is_some() {
                    self.close_wheel();
                }
            }
            HookEvent::MouseDelta(dx, dy) => self.wheel_move(dx, dy),
            HookEvent::DisplayChanged => self.refresh_displays(display::enumerate()),
        }
    }

    fn wheel_items(&self) -> Vec<WheelItem> {
        let s = &self.settings;
        let find = |id: &str| -> Option<WheelItem> {
            if let Some(p) = s.color_presets.iter().find(|p| p.id == id) {
                return Some(WheelItem { id: p.id.clone(), name: p.name.clone(), kind: "color".into() });
            }
            s.gamma_presets
                .iter()
                .find(|p| p.id == id)
                .map(|p| WheelItem { id: p.id.clone(), name: p.name.clone(), kind: "gamma".into() })
        };
        let mut items: Vec<WheelItem> = s.wheel_presets.iter().filter_map(|id| find(id)).take(8).collect();
        if items.is_empty() {
            items = s
                .color_presets
                .iter()
                .take(8)
                .map(|p| WheelItem { id: p.id.clone(), name: p.name.clone(), kind: "color".into() })
                .collect();
        }
        items
    }

    fn open_wheel(&mut self) {
        let items = self.wheel_items();
        if items.is_empty() || self.wheel_hwnd == 0 {
            return;
        }
        let mut pt = windows::Win32::Foundation::POINT::default();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt);
        }
        if let Some(m) = display::monitor_at(pt.x, pt.y) {
            let half = WHEEL_SIZE / 2;
            let cx = m.x + m.width / 2;
            let cy = m.y + m.height / 2;
            window::place(self.wheel_hwnd, cx - half, cy - half, WHEEL_SIZE, WHEEL_SIZE, true);
        }
        self.wheel = Some(Wheel { items: items.clone(), dx: 0.0, dy: 0.0, selected: -1 });
        hooks::TRACK_MOUSE.store(true, Ordering::Relaxed);
        let _ = self.app.emit_to("wheel", "wheel-state", WheelState { visible: true, items, selected: -1 });
    }

    fn wheel_move(&mut self, dx: i32, dy: i32) {
        let Some(w) = self.wheel.as_mut() else { return };
        w.dx += dx as f64;
        w.dy += dy as f64;
        let len = (w.dx * w.dx + w.dy * w.dy).sqrt();
        const MAX: f64 = 180.0;
        if len > MAX {
            w.dx *= MAX / len;
            w.dy *= MAX / len;
        }
        let sel = wheel_index(w.dx, w.dy, w.items.len());
        if sel != w.selected {
            w.selected = sel;
            let state = WheelState { visible: true, items: w.items.clone(), selected: sel };
            let _ = self.app.emit_to("wheel", "wheel-state", state);
        }
    }

    fn close_wheel(&mut self) {
        hooks::TRACK_MOUSE.store(false, Ordering::Relaxed);
        let Some(w) = self.wheel.take() else { return };
        window::place(self.wheel_hwnd, 0, 0, 0, 0, false);
        let _ = self.app.emit_to("wheel", "wheel-state", WheelState { visible: false, items: vec![], selected: -1 });
        if w.selected >= 0 {
            if let Some(item) = w.items.get(w.selected as usize) {
                let id = item.id.clone();
                self.apply_preset(&id);
            }
        }
    }

    /// Runs every second: saves settings, follows monitor changes and puts our
    /// ramps back when Windows or a game reset them.
    fn tick(&mut self) {
        self.ticks += 1;
        if self.dirty {
            if settings::save(&self.settings).is_ok() {
                self.dirty = false;
            }
        }
        if self.ticks % 3 != 0 || self.shut_down {
            return;
        }
        // A safety net for layout changes that arrive without WM_DISPLAYCHANGE.
        let now = display::enumerate();
        if !MonitorInfo::same_layout(&now, &self.monitors) {
            self.refresh_displays(now);
            return;
        }
        let mut drifted = Vec::new();
        for (id, ramp) in &self.applied {
            let strength = self.gamma_strength.get(id).copied().unwrap_or(1.0);
            if strength < 0.999 {
                continue; // a reduced ramp never matches the full one
            }
            if let Some(cur) = display::get_ramp(id) {
                if ramp_distance(&cur, ramp) > 768 {
                    drifted.push(id.clone());
                }
            }
        }
        if !drifted.is_empty() {
            for id in drifted {
                self.applied.remove(&id);
            }
            self.apply_gamma();
        }
    }

    /// Restores every original and removes all effects. Safe to call twice.
    pub fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        self.shut_down = true;
        hooks::TRACK_MOUSE.store(false, Ordering::Relaxed);
        for m in self.monitors.clone() {
            if let Some(o) = self.originals.get(&m.id) {
                display::set_ramp(&m.id, o);
            }
        }
        self.restore_vibrance();
        // Releases the backends, then ADL2_Main_Control_Destroy through AdlLib's drop.
        self.vib.clear();
        self.adl_lib = None;
        mag::shutdown();
        let _ = settings::save(&self.settings);
        let _ = std::fs::remove_file(restore_path());
    }
}

/// Which wheel slice the pointer points at: item 0 at the top, clockwise.
/// -1 while the pointer is still near the centre.
pub fn wheel_index(dx: f64, dy: f64, n: usize) -> i32 {
    if n == 0 || (dx * dx + dy * dy).sqrt() < 36.0 {
        return -1;
    }
    let angle = dx.atan2(-dy); // 0 = up, clockwise positive
    let step = std::f64::consts::TAU / n as f64;
    ((angle / step).round() as i64).rem_euclid(n as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_directions() {
        assert_eq!(wheel_index(0.0, -100.0, 8), 0); // up
        assert_eq!(wheel_index(100.0, 0.0, 8), 2); // right
        assert_eq!(wheel_index(0.0, 100.0, 8), 4); // down
        assert_eq!(wheel_index(-100.0, 0.0, 8), 6); // left
        assert_eq!(wheel_index(70.0, -70.0, 8), 1); // up-right
        assert_eq!(wheel_index(5.0, 5.0, 8), -1); // dead zone
        assert_eq!(wheel_index(0.0, -100.0, 0), -1);
    }

    #[test]
    fn nvg_tones_are_max_gamma() {
        assert_eq!(nvg_tone("green").gamma, 3.0);
        assert!(nvg_tone("green").green > nvg_tone("green").red);
        assert!(nvg_tone("blue").blue > nvg_tone("blue").red);
    }

    #[test]
    fn ramp_vec_roundtrip() {
        let r = gamma::build_ramp(&Tone { gamma: 1.7, ..Tone::default() });
        assert_eq!(vec_to_ramp(&ramp_to_vec(&r)), Some(r));
        assert_eq!(vec_to_ramp(&[vec![0; 3]]), None);
    }
}
