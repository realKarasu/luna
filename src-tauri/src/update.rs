//! Automatic updates.
//!
//! Luna reads `latest.json` from the project's latest GitHub release. When it names a newer
//! version, the installer is downloaded, its signature is checked against the public key built
//! into the app, and the installer runs. Nothing else is sent or received.
//!
//! - First check, a few seconds after start: a found update is installed right away and Luna
//!   starts again by itself.
//! - Later checks, every few hours: the update is downloaded quietly and installed when Luna
//!   quits, or at once from the Restart button, so a game is never interrupted.

use crate::engine::{self, engine};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(8);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// idle, checking, downloading, ready, latest or error.
    pub state: &'static str,
    pub version: Option<String>,
    pub notes: Option<String>,
    /// 0 to 1 while downloading.
    pub progress: f64,
    pub error: Option<String>,
    /// Unix seconds of the last finished check.
    pub checked_at: Option<u64>,
}

impl UpdateStatus {
    const fn idle() -> Self {
        UpdateStatus { state: "idle", version: None, notes: None, progress: 0.0, error: None, checked_at: None }
    }
}

static STATUS: Mutex<UpdateStatus> = Mutex::new(UpdateStatus::idle());
static PENDING: Mutex<Option<(Update, Vec<u8>)>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn status() -> UpdateStatus {
    STATUS.lock().map(|s| s.clone()).unwrap_or_else(|_| UpdateStatus::idle())
}

fn set(app: &AppHandle, f: impl FnOnce(&mut UpdateStatus)) {
    let snapshot = match STATUS.lock() {
        Ok(mut s) => {
            f(&mut s);
            s.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit("update-status", snapshot);
}

fn now() -> Option<u64> {
    SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn has_pending() -> bool {
    PENDING.lock().map(|p| p.is_some()).unwrap_or(false)
}

/// Checks for a newer version and downloads it. Returns true when an update is ready to install.
pub async fn check_and_download(app: &AppHandle) -> bool {
    if has_pending() {
        return true;
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return false;
    }
    let ready = run_check(app).await;
    BUSY.store(false, Ordering::SeqCst);
    ready
}

async fn run_check(app: &AppHandle) -> bool {
    set(app, |s| {
        s.state = "checking";
        s.error = None;
    });
    // The screens go back to normal before the installer replaces Luna.
    let mut builder = app.updater_builder().on_before_exit(restore_screens);
    // For testing a release before it goes out. The signature check still applies, so this
    // can only ever install a build signed with the project's key.
    if let Some(url) = std::env::var("LUNA_UPDATE_ENDPOINT").ok().and_then(|u| u.parse().ok()) {
        builder = match builder.endpoints(vec![url]) {
            Ok(b) => b,
            Err(e) => {
                let msg = e.to_string();
                set(app, |s| {
                    s.state = "error";
                    s.error = Some(msg);
                });
                return false;
            }
        };
    }
    let found = match builder.build() {
        Ok(updater) => updater.check().await,
        Err(e) => Err(e),
    };
    let update = match found {
        Ok(Some(u)) => u,
        Ok(None) => {
            set(app, |s| {
                s.state = "latest";
                s.version = None;
                s.notes = None;
                s.checked_at = now();
            });
            return false;
        }
        Err(e) => {
            let msg = e.to_string();
            set(app, |s| {
                s.state = "error";
                s.error = Some(msg);
                s.checked_at = now();
            });
            return false;
        }
    };

    let version = update.version.clone();
    let notes = update.body.clone().filter(|b| !b.trim().is_empty());
    set(app, |s| {
        s.state = "downloading";
        s.version = Some(version);
        s.notes = notes;
        s.progress = 0.0;
    });

    let mut got: u64 = 0;
    let mut last = Instant::now();
    let progress_app = app.clone();
    let bytes = update
        .download(
            move |chunk, total| {
                got += chunk as u64;
                if let Some(total) = total.filter(|t| *t > 0) {
                    if last.elapsed() >= Duration::from_millis(150) {
                        last = Instant::now();
                        let p = (got as f64 / total as f64).min(1.0);
                        set(&progress_app, |s| s.progress = p);
                    }
                }
            },
            || {},
        )
        .await;

    match bytes {
        Ok(bytes) => {
            if let Ok(mut p) = PENDING.lock() {
                *p = Some((update, bytes));
            }
            set(app, |s| {
                s.state = "ready";
                s.progress = 1.0;
                s.checked_at = now();
            });
            true
        }
        Err(e) => {
            let msg = e.to_string();
            set(app, |s| {
                s.state = "error";
                s.error = Some(msg);
                s.checked_at = now();
            });
            false
        }
    }
}

/// Runs the downloaded installer and ends Luna. The screens are put back first through the
/// updater's before-exit hook. With `restart`, the installer starts Luna again when it is done.
pub fn install_now(restart: bool) -> Result<(), String> {
    let pending = PENDING.lock().map_err(|e| e.to_string())?.take();
    match pending {
        Some((update, bytes)) => update.restart_after_install(restart).install(&bytes).map_err(|e| e.to_string()),
        None => Err("No update is ready".into()),
    }
}

/// Called on every way out of Luna: an update that is waiting gets installed now.
pub fn install_pending_on_quit() {
    if has_pending() {
        let _ = install_now(false);
    }
}

fn restore_screens() {
    if engine::engine_ready() {
        engine().shutdown();
    }
}

fn enabled() -> bool {
    engine::engine_ready() && engine().settings.app.auto_update
}

/// Background checks while Luna runs.
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("luna-update".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK);
            let mut first = true;
            loop {
                if enabled() && tauri::async_runtime::block_on(check_and_download(&app)) && first {
                    // Luna was just opened: install now, it starts again by itself.
                    let _ = install_now(true);
                }
                first = false;
                std::thread::sleep(CHECK_EVERY);
            }
        })
        .ok();
}
