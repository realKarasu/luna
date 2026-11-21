//! Screens and gamma tables through CoreGraphics. Coordinates are points, top-left origin,
//! the same space the mouse position uses. macOS puts the system tables back by itself when
//! Luna ends, whatever the reason.

use crate::gamma::Ramp;
use crate::screens;
pub use crate::screens::MonitorInfo;
pub use crate::screens::MATRIX_VIBRANCE;
use crate::vibrance::Vendor;

type DisplayId = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGGetActiveDisplayList(max: u32, displays: *mut DisplayId, count: *mut u32) -> i32;
    fn CGDisplayBounds(display: DisplayId) -> CGRect;
    fn CGMainDisplayID() -> DisplayId;
    fn CGDisplayIsBuiltin(display: DisplayId) -> u32;
    fn CGDisplayGammaTableCapacity(display: DisplayId) -> u32;
    fn CGGetDisplayTransferByTable(
        display: DisplayId,
        capacity: u32,
        red: *mut f32,
        green: *mut f32,
        blue: *mut f32,
        count: *mut u32,
    ) -> i32;
    fn CGSetDisplayTransferByTable(
        display: DisplayId,
        size: u32,
        red: *const f32,
        green: *const f32,
        blue: *const f32,
    ) -> i32;
}

fn parse(id: &str) -> Option<DisplayId> {
    id.strip_prefix("display-")?.parse().ok()
}

fn active() -> Vec<DisplayId> {
    let mut ids = [0u32; 32];
    let mut count = 0u32;
    let err = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if err != 0 {
        return Vec::new();
    }
    ids[..(count as usize).min(ids.len())].to_vec()
}

/// Every active screen, main screen first, then left to right.
pub fn enumerate() -> Vec<MonitorInfo> {
    let main = unsafe { CGMainDisplayID() };
    let mut list: Vec<MonitorInfo> = active()
        .into_iter()
        .map(|d| {
            let b = unsafe { CGDisplayBounds(d) };
            let builtin = unsafe { CGDisplayIsBuiltin(d) } != 0;
            MonitorInfo {
                id: format!("display-{d}"),
                label: String::new(),
                x: b.origin.x.round() as i32,
                y: b.origin.y.round() as i32,
                width: b.size.width.round() as i32,
                height: b.size.height.round() as i32,
                primary: d == main,
                gpu: String::new(),
                vendor: Vendor::Generic,
                vibrance: MATRIX_VIBRANCE.to_string(),
            }
            .labelled(builtin)
        })
        .collect();
    screens::sort(&mut list);
    let mut n = 0;
    for m in list.iter_mut() {
        if m.label.is_empty() {
            n += 1;
            m.label = format!("Display {n}");
        }
    }
    list
}

trait Labelled {
    fn labelled(self, builtin: bool) -> Self;
}

impl Labelled for MonitorInfo {
    fn labelled(mut self, builtin: bool) -> Self {
        if builtin {
            self.label = "Built-in display".into();
        }
        self
    }
}

pub fn monitor_at(x: i32, y: i32) -> Option<MonitorInfo> {
    screens::monitor_in(enumerate(), x, y)
}

pub fn get_ramp(device: &str) -> Option<Ramp> {
    let d = parse(device)?;
    let cap = unsafe { CGDisplayGammaTableCapacity(d) }.clamp(2, 4096);
    let (mut r, mut g, mut b) = (vec![0f32; cap as usize], vec![0f32; cap as usize], vec![0f32; cap as usize]);
    let mut count = 0u32;
    let err = unsafe { CGGetDisplayTransferByTable(d, cap, r.as_mut_ptr(), g.as_mut_ptr(), b.as_mut_ptr(), &mut count) };
    let n = (count as usize).min(cap as usize);
    if err != 0 || n < 2 {
        return None;
    }
    let mut ramp: Ramp = [[0u16; 256]; 3];
    for (out, ch) in ramp.iter_mut().zip([&r, &g, &b]) {
        let src: Vec<f64> = ch[..n].iter().map(|v| *v as f64).collect();
        for (o, v) in out.iter_mut().zip(screens::resample(&src, 256)) {
            *o = (v.clamp(0.0, 1.0) * 65535.0).round() as u16;
        }
    }
    Some(ramp)
}

pub fn set_ramp(device: &str, ramp: &Ramp) -> bool {
    let Some(d) = parse(device) else { return false };
    let ch = |i: usize| -> Vec<f32> { ramp[i].iter().map(|v| *v as f32 / 65535.0).collect() };
    let (r, g, b) = (ch(0), ch(1), ch(2));
    unsafe { CGSetDisplayTransferByTable(d, 256, r.as_ptr(), g.as_ptr(), b.as_ptr()) == 0 }
}

/// macOS takes any table, so the full effect always applies.
pub fn set_ramp_best_effort(device: &str, base: &Ramp, target: &Ramp) -> Option<f64> {
    screens::best_effort(|r| set_ramp(device, r), base, target)
}
