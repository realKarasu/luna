//! Magnification API: full-screen colour matrix and Ultra Zoom.
//! Every call runs on one dedicated thread that owns `MagInitialize`.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use windows::Win32::UI::Magnification::{
    MagInitialize, MagSetFullscreenColorEffect, MagSetFullscreenTransform, MagUninitialize, MAGCOLOREFFECT,
};

enum Cmd {
    Color([f32; 25]),
    Zoom { level: f32, x: i32, y: i32 },
    Shutdown(Sender<()>),
}

static TX: OnceLock<Mutex<Sender<Cmd>>> = OnceLock::new();
static AVAILABLE: OnceLock<bool> = OnceLock::new();

fn identity() -> [f32; 25] {
    let mut m = [0f32; 25];
    for i in 0..5 {
        m[i * 5 + i] = 1.0;
    }
    m
}

pub fn start() {
    let (tx, rx) = channel::<Cmd>();
    let (ready_tx, ready_rx) = channel::<bool>();
    std::thread::Builder::new()
        .name("luna-magnifier".into())
        .spawn(move || {
            let ok = unsafe { MagInitialize().as_bool() };
            let _ = ready_tx.send(ok);
            let mut last_color = identity();
            let mut zoomed = false;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Cmd::Color(m) => {
                        if ok && m != last_color {
                            let effect = MAGCOLOREFFECT { transform: m };
                            if unsafe { MagSetFullscreenColorEffect(&effect).as_bool() } {
                                last_color = m;
                            }
                        }
                    }
                    Cmd::Zoom { level, x, y } => {
                        if ok && (level > 1.0 || zoomed) {
                            unsafe {
                                let _ = MagSetFullscreenTransform(level.max(1.0), x, y);
                            }
                            zoomed = level > 1.0;
                        }
                    }
                    Cmd::Shutdown(done) => {
                        if ok {
                            unsafe {
                                let effect = MAGCOLOREFFECT { transform: identity() };
                                let _ = MagSetFullscreenColorEffect(&effect);
                                let _ = MagSetFullscreenTransform(1.0, 0, 0);
                                let _ = MagUninitialize();
                            }
                        }
                        let _ = done.send(());
                        break;
                    }
                }
            }
        })
        .expect("magnifier thread");
    let ok = ready_rx.recv().unwrap_or(false);
    let _ = AVAILABLE.set(ok);
    let _ = TX.set(Mutex::new(tx));
}

pub fn available() -> bool {
    *AVAILABLE.get().unwrap_or(&false)
}

fn send(cmd: Cmd) {
    if let Some(tx) = TX.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(cmd);
        }
    }
}

pub fn set_color(m: [f32; 25]) {
    send(Cmd::Color(m));
}

/// `level` 1.0 removes the zoom. `x`, `y` are the top-left of the magnified area
/// in desktop pixels.
pub fn set_zoom(level: f32, x: i32, y: i32) {
    send(Cmd::Zoom { level, x, y });
}

/// Removes the colour effect and zoom, waits up to a second.
pub fn shutdown() {
    let (done_tx, done_rx) = channel();
    send(Cmd::Shutdown(done_tx));
    let _ = done_rx.recv_timeout(std::time::Duration::from_secs(1));
}
