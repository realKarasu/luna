//! Potato Mode: everything a slow PC can give to one game, undone when it stops.
//!
//! It turns on the Optimizer switches that cost nothing to undo (power plan, Game Mode,
//! no background recording, no transparency, no animations), but only those that are off,
//! and only those go back. The game, or the program in front, gets high priority; other
//! programs in your session that run at normal priority drop to below normal with Windows'
//! efficiency mode. Voice, streaming and system programs are left alone. What Luna changed
//! is written to potato.json, so a crash is undone at the next start.

use super::games::{exe_of, foreground_pid, processes};
use super::optimizer;
use crate::games::PotatoStatus;
use crate::settings::{data_dir, write_json_atomic};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex, MutexGuard, TryLockError};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{
    GetPriorityClass, OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
    BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS, PROCESS_CREATION_FLAGS,
    PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
};

/// Optimizer switches Potato Mode turns on.
const TWEAKS: [&str; 5] = ["power", "gameMode", "gameDvr", "transparency", "animations"];

/// Programs never slowed down: Windows itself, Luna, voice chat, streaming and music,
/// and the graphics drivers' own recording.
const KEEP: [&str; 40] = [
    "system",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "dwm.exe",
    "explorer.exe",
    "audiodg.exe",
    "fontdrvhost.exe",
    "sihost.exe",
    "ctfmon.exe",
    "conhost.exe",
    "taskmgr.exe",
    "textinputhost.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "runtimebroker.exe",
    "luna.exe",
    "discord.exe",
    "discordptb.exe",
    "discordcanary.exe",
    "vesktop.exe",
    "teamspeak.exe",
    "ts3client_win64.exe",
    "ts3client_win32.exe",
    "mumble.exe",
    "obs64.exe",
    "obs32.exe",
    "streamlabs obs.exe",
    "spotify.exe",
    "amdrsserv.exe",
    "radeonsoftware.exe",
    "nvidia share.exe",
    "nvidia overlay.exe",
    "nvcontainer.exe",
];

#[derive(Clone, Serialize, Deserialize)]
struct Touched {
    pid: u32,
    exe: String,
    /// Priority class before Luna.
    original: u32,
    /// Priority class Luna set.
    set: u32,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    tweaks: Vec<String>,
    procs: Vec<Touched>,
}

#[derive(Default)]
struct State {
    manual: bool,
    /// The game whose profile asked for Potato Mode, lower case.
    game: Option<String>,
    on: bool,
    tweaks: Vec<String>,
    procs: HashMap<u32, Touched>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn file() -> std::path::PathBuf {
    data_dir().join("potato.json")
}

fn save(s: &State) {
    if s.tweaks.is_empty() && s.procs.is_empty() {
        let _ = std::fs::remove_file(file());
        return;
    }
    let saved = Saved { tweaks: s.tweaks.clone(), procs: s.procs.values().cloned().collect() };
    let _ = write_json_atomic(&file(), &saved);
}

struct Proc(HANDLE);

impl Proc {
    fn open(pid: u32) -> Option<Proc> {
        unsafe { OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok().map(Proc) }
    }

    fn class(&self) -> u32 {
        unsafe { GetPriorityClass(self.0) }
    }

    /// Priority class, and efficiency mode on (`Some(true)`), off (`Some(false)`) or left to Windows.
    fn set(&self, class: u32, eco: Option<bool>) -> bool {
        unsafe {
            let ok = SetPriorityClass(self.0, PROCESS_CREATION_FLAGS(class)).is_ok();
            let (control, state) = match eco {
                Some(true) => (PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_EXECUTION_SPEED),
                Some(false) => (PROCESS_POWER_THROTTLING_EXECUTION_SPEED, 0),
                None => (0, 0),
            };
            let t = PROCESS_POWER_THROTTLING_STATE {
                Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                ControlMask: control,
                StateMask: state,
            };
            let _ = SetProcessInformation(
                self.0,
                ProcessPowerThrottling,
                &t as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            );
            ok
        }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn session_of(pid: u32) -> Option<u32> {
    let mut sid = 0u32;
    unsafe { ProcessIdToSessionId(pid, &mut sid).ok().map(|_| sid) }
}

/// Puts one program back, if it is still the same program.
fn undo(t: &Touched, running: &HashMap<u32, String>) {
    if running.get(&t.pid) != Some(&t.exe) {
        return;
    }
    if let Some(p) = Proc::open(t.pid) {
        if p.class() == t.set {
            p.set(t.original, None);
        }
    }
}

fn turn_on(s: &mut State) {
    s.on = true;
    let status = optimizer::status();
    for id in TWEAKS {
        let off = status.iter().any(|t| t.id == id && t.available && !t.on);
        if off && optimizer::set(id, true).is_ok() {
            s.tweaks.push(id.to_string());
        }
    }
    save(s);
}

fn turn_off(s: &mut State) {
    s.on = false;
    for id in std::mem::take(&mut s.tweaks).iter().rev() {
        let _ = optimizer::set(id, false);
    }
    let running: HashMap<u32, String> = processes().into_iter().collect();
    for t in std::mem::take(&mut s.procs).values() {
        undo(t, &running);
    }
    save(s);
}

fn sync(s: &mut State, keep: &[String]) {
    let want = s.manual || s.game.is_some();
    if want && !s.on {
        turn_on(s);
        apply_priorities(s, keep);
    } else if !want && s.on {
        turn_off(s);
    }
}

/// Gives the game high priority and slows the rest. Runs again every few seconds
/// for programs that started since, and when the program in front changes.
fn apply_priorities(s: &mut State, keep: &[String]) {
    let me = std::process::id();
    let my_session = session_of(me);
    let procs = processes();
    let running: HashMap<u32, String> = procs.iter().cloned().collect();
    s.procs.retain(|pid, t| running.get(pid) == Some(&t.exe));

    let boost: HashSet<u32> = match &s.game {
        Some(game) => procs.iter().filter(|(_, exe)| exe == game).map(|(pid, _)| *pid).collect(),
        None => {
            let fg = foreground_pid();
            let exe = exe_of(fg);
            if fg == 0 || fg == me || exe.is_empty() || KEEP.contains(&exe.as_str()) {
                HashSet::new()
            } else {
                HashSet::from([fg])
            }
        }
    };
    let keep: HashSet<String> = keep.iter().map(|k| k.to_lowercase()).collect();
    let high = HIGH_PRIORITY_CLASS.0;
    let low = BELOW_NORMAL_PRIORITY_CLASS.0;

    for (pid, exe) in &procs {
        let boosted = boost.contains(pid);
        if *pid == me || *pid <= 4 || (!boosted && (KEEP.contains(&exe.as_str()) || keep.contains(exe))) {
            continue;
        }
        let want = if boosted { high } else { low };
        if let Some(t) = s.procs.get_mut(pid) {
            if t.set != want {
                if let Some(p) = Proc::open(*pid) {
                    if p.set(want, Some(!boosted)) {
                        t.set = want;
                    }
                }
            }
            continue;
        }
        // Services and other people's programs are not ours to slow down.
        if my_session.is_none() || session_of(*pid) != my_session {
            continue;
        }
        let Some(p) = Proc::open(*pid) else { continue };
        let original = p.class();
        // Someone chose another priority for this program: keep it.
        if original != NORMAL_PRIORITY_CLASS.0 {
            continue;
        }
        if p.set(want, Some(!boosted)) {
            s.procs.insert(*pid, Touched { pid: *pid, exe: exe.clone(), original, set: want });
        }
    }
    save(s);
}

fn status_of(s: &State) -> PotatoStatus {
    let high = HIGH_PRIORITY_CLASS.0;
    PotatoStatus {
        on: s.on,
        manual: s.manual,
        game: s.game.clone(),
        tweaks: s.tweaks.clone(),
        slowed: s.procs.values().filter(|t| t.set != high).count(),
        boosted: s.procs.values().find(|t| t.set == high).map(|t| t.exe.clone()),
    }
}

pub fn status() -> PotatoStatus {
    status_of(&state())
}

pub fn active() -> bool {
    state().on
}

/// The switch on the Games page and the Potato key.
pub fn set_manual(on: bool, keep: &[String]) -> Result<PotatoStatus, String> {
    let mut s = state();
    s.manual = on;
    sync(&mut s, keep);
    Ok(status_of(&s))
}

/// A game whose profile asks for Potato Mode started (`Some`) or closed (`None`).
pub fn set_game(exe: Option<&str>, keep: &[String]) {
    let mut s = state();
    s.game = exe.map(str::to_lowercase);
    sync(&mut s, keep);
    if s.on {
        apply_priorities(&mut s, keep);
    }
}

/// New programs and a new program in front.
pub fn refresh(keep: &[String]) {
    let mut s = state();
    if s.on {
        apply_priorities(&mut s, keep);
    }
}

/// Everything back, for quitting.
pub fn stop() {
    let mut s = state();
    s.manual = false;
    s.game = None;
    if s.on {
        turn_off(&mut s);
    }
}

/// Priorities back from the panic hook; the switches come back at the next start.
pub fn stop_after_panic() {
    let guard = match STATE.try_lock() {
        Ok(g) => g,
        Err(TryLockError::Poisoned(p)) => p.into_inner(),
        Err(TryLockError::WouldBlock) => return,
    };
    let running: HashMap<u32, String> = processes().into_iter().collect();
    for t in guard.procs.values() {
        undo(t, &running);
    }
}

/// Undoes what the last run left behind if it did not quit cleanly.
pub fn recover() {
    let Ok(text) = std::fs::read_to_string(file()) else { return };
    let saved: Saved = serde_json::from_str(&text).unwrap_or_default();
    for id in saved.tweaks.iter().rev() {
        let _ = optimizer::set(id, false);
    }
    let running: HashMap<u32, String> = processes().into_iter().collect();
    for t in &saved.procs {
        undo(t, &running);
    }
    let _ = std::fs::remove_file(file());
}
