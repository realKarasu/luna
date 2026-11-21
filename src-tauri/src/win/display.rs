//! Monitor list and per-monitor gamma ramps (GDI).

use crate::gamma::{self, Ramp};
use crate::vibrance::Vendor;
use serde::Serialize;
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFO,
    MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

const MONITORINFOF_PRIMARY: u32 = 1;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// GDI device name such as `\\.\DISPLAY1`.
    pub id: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
    /// The graphics card driving this monitor, e.g. "AMD Radeon RX 7800 XT".
    pub gpu: String,
    /// "nvidia", "amd", "intel" or "generic".
    pub vendor: Vendor,
    /// What gives this monitor its vibrance, e.g. "AMD saturation" or "Color matrix".
    pub vibrance: String,
}

pub const MATRIX_VIBRANCE: &str = "Color matrix";

impl MonitorInfo {
    /// Same place and size: GPU details are filled in later and do not count.
    pub fn same_layout(a: &[MonitorInfo], b: &[MonitorInfo]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.id == y.id
                    && x.x == y.x
                    && x.y == y.y
                    && x.width == y.width
                    && x.height == y.height
                    && x.primary == y.primary
            })
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn info_for(hmon: HMONITOR) -> Option<MonitorInfo> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    let ok = unsafe { GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) };
    if !ok.as_bool() {
        return None;
    }
    let len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
    let id = String::from_utf16_lossy(&info.szDevice[..len]);
    let r = info.monitorInfo.rcMonitor;
    let number: String = id.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    let width = r.right - r.left;
    let height = r.bottom - r.top;
    let label = if number.is_empty() { id.clone() } else { format!("Display {number}") };
    Some(MonitorInfo {
        id,
        label,
        x: r.left,
        y: r.top,
        width,
        height,
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        gpu: String::new(),
        vendor: Vendor::Generic,
        vibrance: MATRIX_VIBRANCE.to_string(),
    })
}

unsafe extern "system" fn enum_proc(hmon: HMONITOR, _hdc: HDC, _rect: *mut RECT, data: LPARAM) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<MonitorInfo>);
    if let Some(m) = info_for(hmon) {
        list.push(m);
    }
    BOOL(1)
}

/// Every active monitor, primary first, then left to right.
pub fn enumerate() -> Vec<MonitorInfo> {
    let mut list: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(&mut list as *mut _ as isize));
    }
    list.sort_by_key(|m| (!m.primary, m.x, m.y));
    list
}

/// The monitor that contains the point (or the nearest one).
pub fn monitor_at(x: i32, y: i32) -> Option<MonitorInfo> {
    let hmon = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
    info_for(hmon)
}

struct Dc(HDC);

impl Dc {
    fn open(device: &str) -> Option<Dc> {
        let name = wide(device);
        let hdc = unsafe { CreateDCW(PCWSTR::null(), PCWSTR(name.as_ptr()), PCWSTR::null(), None) };
        if hdc.is_invalid() {
            None
        } else {
            Some(Dc(hdc))
        }
    }
}

impl Drop for Dc {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteDC(self.0);
        }
    }
}

pub fn get_ramp(device: &str) -> Option<Ramp> {
    let dc = Dc::open(device)?;
    let mut ramp: Ramp = [[0u16; 256]; 3];
    let ok = unsafe { GetDeviceGammaRamp(dc.0, ramp.as_mut_ptr() as *mut core::ffi::c_void) };
    if ok.as_bool() {
        Some(ramp)
    } else {
        None
    }
}

pub fn set_ramp(device: &str, ramp: &Ramp) -> bool {
    let Some(dc) = Dc::open(device) else { return false };
    unsafe { SetDeviceGammaRamp(dc.0, ramp.as_ptr() as *const core::ffi::c_void).as_bool() }
}

/// Sets `target`; if Windows refuses it (it limits how far a ramp may stray from
/// identity unless the machine-wide gamma range was unlocked), finds the strongest
/// mix between `base` and `target` that Windows accepts.
/// Returns the fraction of the effect that was applied (0..1), or None on failure.
pub fn set_ramp_best_effort(device: &str, base: &Ramp, target: &Ramp) -> Option<f64> {
    if set_ramp(device, target) {
        return Some(1.0);
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let mut best: Option<f64> = None;
    for _ in 0..7 {
        let mid = (lo + hi) / 2.0;
        let r = gamma::blend_ramp(base, target, mid);
        if set_ramp(device, &r) {
            best = Some(mid);
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if let Some(t) = best {
        // Leave the last accepted mix in place.
        let _ = set_ramp(device, &gamma::blend_ramp(base, target, t));
    }
    best
}
