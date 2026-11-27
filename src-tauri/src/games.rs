//! Game profiles, Potato Mode and the FPS counter: two background threads.
//!
//! The first one looks at the running programs every two seconds. When a game with a
//! profile starts, the engine puts on its look (gamma, colour, crosshair), and this thread
//! sets its screen mode and Potato Mode; when it closes everything goes back. The second
//! one reads the FPS counter four times a second and sends it to its overlay.

use crate::engine::{engine, engine_ready, GameStep};
use crate::plat;
use serde::Serialize;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PotatoStatus {
    pub on: bool,
    /// Turned on by hand (switch or key).
    pub manual: bool,
    /// The game whose profile turned it on.
    pub game: Option<String>,
    /// Optimizer switches Luna turned on for it.
    pub tweaks: Vec<String>,
    /// Programs slowed down right now.
    pub slowed: usize,
    /// The program with high priority.
    pub boosted: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FpsReading {
    /// "running", "needsAdmin" or "error".
    pub state: String,
    /// 0 when the program in front draws nothing.
    pub fps: f32,
    pub ms: f32,
    /// 1% low.
    pub low: f32,
    pub exe: String,
    /// The last frame times in milliseconds, oldest first, when the graph is on.
    pub gaps: Vec<f32>,
    /// Processor, graphics card and memory, when the details are on.
    pub sys: Option<SysReading>,
}

/// What the PC is doing. A value Windows or the driver does not give stays empty.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysReading {
    /// Processor load, 0 to 100.
    pub cpu: Option<f32>,
    /// Graphics card load, 0 to 100.
    pub gpu: Option<f32>,
    /// Graphics card temperature in degrees Celsius.
    pub gpu_temp: Option<f32>,
    /// Video memory in use, in GB.
    pub vram: Option<f32>,
    /// Memory in use, in GB.
    pub ram: Option<f32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveGame {
    pub exe: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamesStatus {
    pub active: Option<ActiveGame>,
    pub potato: PotatoStatus,
    pub fps: Option<FpsReading>,
}

static LAST_FPS: LazyLock<Mutex<Option<FpsReading>>> = LazyLock::new(|| Mutex::new(None));

/// Programs with a profile: Potato Mode never slows them down.
fn profile_exes() -> Vec<String> {
    engine().settings.games.iter().map(|g| g.exe.to_lowercase()).collect()
}

pub fn status() -> GamesStatus {
    let active = engine().active_game();
    GamesStatus {
        active,
        potato: plat::potato::status(),
        fps: LAST_FPS.lock().unwrap_or_else(|e| e.into_inner()).clone(),
    }
}

/// The Potato switch on the Games page and the Potato key.
pub fn set_potato(on: bool) -> Result<GamesStatus, String> {
    let keep = profile_exes();
    plat::potato::set_manual(on, &keep)?;
    Ok(status())
}

pub fn start(app: AppHandle) {
    let watch = app.clone();
    let _ = std::thread::Builder::new().name("luna-games".into()).spawn(move || {
        let mut n: u64 = 0;
        loop {
            std::thread::sleep(Duration::from_secs(2));
            if !engine_ready() {
                continue;
            }
            n += 1;
            let running = plat::games::running();
            let (step, keep) = {
                let mut e = engine();
                let keep: Vec<String> = e.settings.games.iter().map(|g| g.exe.to_lowercase()).collect();
                (e.game_step(&running), keep)
            };
            match step {
                GameStep::Started { exe, modes, potato } => {
                    for (i, (monitor, mode)) in modes.iter().enumerate() {
                        if let Err(err) = plat::games::set_mode(monitor, mode, i > 0) {
                            let _ = watch.emit("game-error", err);
                        }
                    }
                    if potato {
                        plat::potato::set_game(Some(&exe), &keep);
                    }
                    let _ = watch.emit("game-changed", status());
                }
                GameStep::Ended { screens, potato } => {
                    plat::games::restore_modes(&screens);
                    if potato {
                        plat::potato::set_game(None, &keep);
                    }
                    let _ = watch.emit("game-changed", status());
                }
                GameStep::Same => {
                    // New programs and a new program in front, every four seconds.
                    if n % 2 == 0 && plat::potato::active() {
                        plat::potato::refresh(&keep);
                    }
                }
            }
        }
    });

    let _ = std::thread::Builder::new().name("luna-fps".into()).spawn(move || {
        let mut shown = String::from("off");
        let mut sampler = plat::sysmon::Sampler::default();
        let mut sys = None;
        let mut n: u64 = 0;
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let mut reading = plat::fps::poll();
            // The system numbers move slowly: once a second is enough.
            if n % 4 == 0 {
                sys = sampler.sample();
            }
            n += 1;
            if let Some(r) = &mut reading {
                r.sys = sys.clone();
            }
            *LAST_FPS.lock().unwrap_or_else(|e| e.into_inner()) = reading.clone();
            let state = reading.as_ref().map_or("off", |r| r.state.as_str()).to_string();
            if let Some(r) = &reading {
                let _ = app.emit_to("fps", "fps", r);
            }
            if state != shown {
                // The Games page shows it: running, needs administrator, off.
                let _ = app.emit("fps-state", &state);
                shown = state;
            }
        }
    });
}
