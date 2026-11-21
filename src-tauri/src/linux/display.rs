//! Screens and gamma tables through XRandR (libX11 and libXrandr, loaded at run time).
//! Positions and sizes are X pixels, the same space the mouse position uses. Under Wayland
//! this only reaches XWayland, which does not change the real screen.

use crate::gamma::Ramp;
use crate::screens;
pub use crate::screens::MonitorInfo;
pub use crate::screens::MATRIX_VIBRANCE;
use crate::vibrance::Vendor;
use std::os::raw::c_int;
use std::sync::Mutex;
use x11_dl::xlib::{Display, Window, Xlib};
use x11_dl::xrandr::{RRCrtc, RR_Connected, Xrandr};

struct XConn {
    xlib: Xlib,
    xrr: Xrandr,
    dpy: *mut Display,
    root: Window,
}

// The connection is only ever used behind the mutex, one call at a time.
unsafe impl Send for XConn {}

/// `None` until the first try; `Some(None)` when there is no X server to talk to.
static CONN: Mutex<Option<Option<XConn>>> = Mutex::new(None);

fn connect() -> Option<XConn> {
    let xlib = Xlib::open().ok()?;
    let xrr = Xrandr::open().ok()?;
    unsafe {
        let dpy = (xlib.XOpenDisplay)(std::ptr::null());
        if dpy.is_null() {
            return None;
        }
        let root = (xlib.XDefaultRootWindow)(dpy);
        Some(XConn { xlib, xrr, dpy, root })
    }
}

fn with<R>(f: impl FnOnce(&XConn) -> R) -> Option<R> {
    let mut guard = CONN.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(connect());
    }
    guard.as_ref().and_then(|c| c.as_ref()).map(f)
}

struct Output {
    name: String,
    crtc: RRCrtc,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    primary: bool,
}

/// Connected outputs that show a picture, read fresh from the server.
fn outputs(c: &XConn) -> Vec<Output> {
    let mut list = Vec::new();
    unsafe {
        let res = (c.xrr.XRRGetScreenResourcesCurrent)(c.dpy, c.root);
        if res.is_null() {
            return list;
        }
        let primary = (c.xrr.XRRGetOutputPrimary)(c.dpy, c.root);
        let r = &*res;
        let outs = if r.outputs.is_null() || r.noutput <= 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(r.outputs, r.noutput as usize)
        };
        for &o in outs {
            let info = (c.xrr.XRRGetOutputInfo)(c.dpy, res, o);
            if info.is_null() {
                continue;
            }
            let i = &*info;
            if i.connection as c_int == RR_Connected && i.crtc != 0 {
                let crtc = (c.xrr.XRRGetCrtcInfo)(c.dpy, res, i.crtc);
                if !crtc.is_null() {
                    let ci = &*crtc;
                    if ci.width > 0 && ci.height > 0 && !i.name.is_null() && i.nameLen > 0 {
                        let bytes = std::slice::from_raw_parts(i.name as *const u8, i.nameLen as usize);
                        list.push(Output {
                            name: String::from_utf8_lossy(bytes).into_owned(),
                            crtc: i.crtc,
                            x: ci.x,
                            y: ci.y,
                            width: ci.width as i32,
                            height: ci.height as i32,
                            primary: o == primary,
                        });
                    }
                    (c.xrr.XRRFreeCrtcInfo)(crtc);
                }
            }
            (c.xrr.XRRFreeOutputInfo)(info);
        }
        (c.xrr.XRRFreeScreenResources)(res);
    }
    list
}

fn crtc_of(c: &XConn, device: &str) -> Option<RRCrtc> {
    outputs(c).into_iter().find(|o| o.name == device).map(|o| o.crtc)
}

/// Every screen that shows a picture, primary first, then left to right.
pub fn enumerate() -> Vec<MonitorInfo> {
    let outs = with(outputs).unwrap_or_default();
    let any_primary = outs.iter().any(|o| o.primary);
    let mut list: Vec<MonitorInfo> = outs
        .into_iter()
        .enumerate()
        .map(|(i, o)| MonitorInfo {
            label: o.name.clone(),
            id: o.name,
            x: o.x,
            y: o.y,
            width: o.width,
            height: o.height,
            primary: o.primary || (!any_primary && i == 0),
            gpu: String::new(),
            vendor: Vendor::Generic,
            vibrance: MATRIX_VIBRANCE.to_string(),
        })
        .collect();
    screens::sort(&mut list);
    for (n, m) in list.iter_mut().enumerate() {
        m.label = format!("Display {} ({})", n + 1, m.label);
    }
    list
}

pub fn monitor_at(x: i32, y: i32) -> Option<MonitorInfo> {
    screens::monitor_in(enumerate(), x, y)
}

pub fn get_ramp(device: &str) -> Option<Ramp> {
    with(|c| unsafe {
        let crtc = crtc_of(c, device)?;
        let size = (c.xrr.XRRGetCrtcGammaSize)(c.dpy, crtc);
        if size < 2 {
            return None;
        }
        let g = (c.xrr.XRRGetCrtcGamma)(c.dpy, crtc);
        if g.is_null() {
            return None;
        }
        let gr = &*g;
        let n = gr.size.min(size).max(0) as usize;
        let mut ramp: Ramp = [[0u16; 256]; 3];
        let ok = n >= 2 && !gr.red.is_null() && !gr.green.is_null() && !gr.blue.is_null();
        if ok {
            for (out, ptr) in ramp.iter_mut().zip([gr.red, gr.green, gr.blue]) {
                let src: Vec<f64> = std::slice::from_raw_parts(ptr, n).iter().map(|v| *v as f64).collect();
                for (o, v) in out.iter_mut().zip(screens::resample(&src, 256)) {
                    *o = v.round().clamp(0.0, 65535.0) as u16;
                }
            }
        }
        (c.xrr.XRRFreeGamma)(g);
        ok.then_some(ramp)
    })
    .flatten()
}

pub fn set_ramp(device: &str, ramp: &Ramp) -> bool {
    with(|c| unsafe {
        // The CRTC and its table size are read right before the write, so a screen that
        // was just unplugged is skipped instead of raising an X error.
        let Some(crtc) = crtc_of(c, device) else { return false };
        let size = (c.xrr.XRRGetCrtcGammaSize)(c.dpy, crtc);
        if size < 2 {
            return false;
        }
        let g = (c.xrr.XRRAllocGamma)(size);
        if g.is_null() {
            return false;
        }
        let gr = &mut *g;
        for (ch, ptr) in ramp.iter().zip([gr.red, gr.green, gr.blue]) {
            let src: Vec<f64> = ch.iter().map(|v| *v as f64).collect();
            let dst = std::slice::from_raw_parts_mut(ptr, size as usize);
            for (d, v) in dst.iter_mut().zip(screens::resample(&src, size as usize)) {
                *d = v.round().clamp(0.0, 65535.0) as u16;
            }
        }
        (c.xrr.XRRSetCrtcGamma)(c.dpy, crtc, g);
        (c.xrr.XRRFreeGamma)(g);
        (c.xlib.XFlush)(c.dpy);
        true
    })
    .unwrap_or(false)
}

/// X takes any table, so the full effect always applies.
pub fn set_ramp_best_effort(device: &str, base: &Ramp, target: &Ramp) -> Option<f64> {
    screens::best_effort(|r| set_ramp(device, r), base, target)
}
