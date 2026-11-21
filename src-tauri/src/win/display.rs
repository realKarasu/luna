//! Monitor list and per-monitor gamma ramps (GDI).

use crate::gamma::Ramp;
use crate::screens;
use crate::vibrance::Vendor;
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFO,
    MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

const MONITORINFOF_PRIMARY: u32 = 1;

pub use crate::screens::{MonitorInfo, MATRIX_VIBRANCE};

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
    screens::sort(&mut list);
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

/// Sets `target`, or the strongest part of it that Windows accepts (see `screens::best_effort`).
pub fn set_ramp_best_effort(device: &str, base: &Ramp, target: &Ramp) -> Option<f64> {
    screens::best_effort(|r| set_ramp(device, r), base, target)
}
