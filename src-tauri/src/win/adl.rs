//! AMD saturation through the AMD Display Library, loaded at run time from
//! atiadlxx.dll (or atiadlxy.dll) in System32. Nothing is linked: on a PC
//! without an AMD driver, or if any call fails, Luna uses the colour matrix.

use crate::vibrance::{VibranceBackend, VibranceRange};
use std::ffi::c_void;
use std::sync::Arc;
use windows::core::{s, w, PCSTR};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};
use windows::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree, HEAP_FLAGS, HEAP_ZERO_MEMORY};

const ADL_OK: i32 = 0;
// adl_defines.h: BRIGHTNESS 1 << 0, CONTRAST 1 << 1, SATURATION 1 << 2, HUE 1 << 3.
const ADL_DISPLAY_COLOR_SATURATION: i32 = 1 << 2;
const ADL_DISPLAY_CONNECTED: i32 = 0x0001;
const ADL_DISPLAY_MAPPED: i32 = 0x0002;
const ADL_MAX_PATH: usize = 256;

type Ctx = *mut c_void;
type MallocCallback = unsafe extern "system" fn(i32) -> *mut c_void;
type Create = unsafe extern "C" fn(MallocCallback, i32, *mut Ctx) -> i32;
type Destroy = unsafe extern "C" fn(Ctx) -> i32;
type NumAdapters = unsafe extern "C" fn(Ctx, *mut i32) -> i32;
type AdapterInfoGet = unsafe extern "C" fn(Ctx, *mut AdapterInfo, i32) -> i32;
type DisplayInfoGet = unsafe extern "C" fn(Ctx, i32, *mut i32, *mut *mut DisplayInfo, i32) -> i32;
type ColorGet = unsafe extern "C" fn(Ctx, i32, i32, i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> i32;
type ColorSet = unsafe extern "C" fn(Ctx, i32, i32, i32, i32) -> i32;
type PmLogGet = unsafe extern "C" fn(Ctx, i32, *mut PmLogData) -> i32;

/// ADL_PMLOG_TEMPERATURE_EDGE: the temperature the driver shows for the card.
const PMLOG_TEMPERATURE_EDGE: usize = 8;

/// ADL `ADLPMLogDataOutput`: for each sensor, whether the card has it and its value.
#[repr(C)]
struct PmLogData {
    size: i32,
    sensors: [[i32; 2]; 256],
}

/// ADL `AdapterInfo` as laid out on Windows.
#[repr(C)]
#[derive(Clone, Copy)]
struct AdapterInfo {
    size: i32,
    adapter_index: i32,
    udid: [u8; ADL_MAX_PATH],
    bus_number: i32,
    device_number: i32,
    function_number: i32,
    vendor_id: i32,
    adapter_name: [u8; ADL_MAX_PATH],
    display_name: [u8; ADL_MAX_PATH],
    present: i32,
    exist: i32,
    driver_path: [u8; ADL_MAX_PATH],
    driver_path_ext: [u8; ADL_MAX_PATH],
    pnp_string: [u8; ADL_MAX_PATH],
    os_display_index: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayId {
    logical_index: i32,
    physical_index: i32,
    logical_adapter_index: i32,
    physical_adapter_index: i32,
}

/// ADL `ADLDisplayInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
struct DisplayInfo {
    id: DisplayId,
    controller_index: i32,
    name: [u8; ADL_MAX_PATH],
    manufacturer: [u8; ADL_MAX_PATH],
    display_type: i32,
    output_type: i32,
    connector: i32,
    info_mask: i32,
    info_value: i32,
}

/// ADL hands its buffers back allocated through this callback; they are
/// released with `heap_free`.
unsafe extern "system" fn adl_malloc(size: i32) -> *mut c_void {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    match GetProcessHeap() {
        Ok(heap) => HeapAlloc(heap, HEAP_ZERO_MEMORY, size as usize),
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe fn heap_free(p: *mut c_void) {
    if !p.is_null() {
        if let Ok(heap) = GetProcessHeap() {
            let _ = HeapFree(heap, HEAP_FLAGS(0), Some(p as *const c_void));
        }
    }
}

fn c_text(buf: &[u8]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).trim().to_string()
}

/// A loaded ADL with its context. Dropping it calls ADL2_Main_Control_Destroy.
pub struct AdlLib {
    ctx: Ctx,
    destroy: Destroy,
    num_adapters: NumAdapters,
    adapter_info: AdapterInfoGet,
    display_info: DisplayInfoGet,
    color_get: ColorGet,
    color_set: ColorSet,
    /// Missing on drivers from before 2020.
    pmlog: Option<PmLogGet>,
}

// The context is only used behind the engine mutex, one call at a time.
unsafe impl Send for AdlLib {}
unsafe impl Sync for AdlLib {}

impl Drop for AdlLib {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.ctx);
        }
    }
}

unsafe fn proc_of(lib: HMODULE, name: PCSTR) -> Option<*const c_void> {
    GetProcAddress(lib, name).map(|f| f as *const c_void)
}

impl AdlLib {
    /// Loads ADL and creates a context. None when there is no AMD driver.
    pub fn load() -> Option<Arc<AdlLib>> {
        unsafe {
            // Only the copies in System32, never a file placed next to Luna.
            let lib = LoadLibraryExW(w!("atiadlxx.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32)
                .or_else(|_| LoadLibraryExW(w!("atiadlxy.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32))
                .ok()?;
            let create: Create = std::mem::transmute(proc_of(lib, s!("ADL2_Main_Control_Create"))?);
            let destroy: Destroy = std::mem::transmute(proc_of(lib, s!("ADL2_Main_Control_Destroy"))?);
            let num_adapters: NumAdapters =
                std::mem::transmute(proc_of(lib, s!("ADL2_Adapter_NumberOfAdapters_Get"))?);
            let adapter_info: AdapterInfoGet =
                std::mem::transmute(proc_of(lib, s!("ADL2_Adapter_AdapterInfo_Get"))?);
            let display_info: DisplayInfoGet =
                std::mem::transmute(proc_of(lib, s!("ADL2_Display_DisplayInfo_Get"))?);
            let color_get: ColorGet = std::mem::transmute(proc_of(lib, s!("ADL2_Display_Color_Get"))?);
            let color_set: ColorSet = std::mem::transmute(proc_of(lib, s!("ADL2_Display_Color_Set"))?);
            let mut ctx: Ctx = std::ptr::null_mut();
            if create(adl_malloc, 1, &mut ctx) != ADL_OK || ctx.is_null() {
                return None;
            }
            let pmlog: Option<PmLogGet> =
                proc_of(lib, s!("ADL2_New_QueryPMLogData_Get")).map(|p| std::mem::transmute(p));
            Some(Arc::new(AdlLib { ctx, destroy, num_adapters, adapter_info, display_info, color_get, color_set, pmlog }))
        }
    }

    fn adapters(&self) -> Vec<AdapterInfo> {
        unsafe {
            let mut n = 0i32;
            if (self.num_adapters)(self.ctx, &mut n) != ADL_OK || n <= 0 || n > 256 {
                return Vec::new();
            }
            let mut list: Vec<AdapterInfo> = vec![std::mem::zeroed(); n as usize];
            for a in list.iter_mut() {
                a.size = std::mem::size_of::<AdapterInfo>() as i32;
            }
            let bytes = (std::mem::size_of::<AdapterInfo>() * n as usize) as i32;
            if (self.adapter_info)(self.ctx, list.as_mut_ptr(), bytes) != ADL_OK {
                return Vec::new();
            }
            list
        }
    }

    /// Temperature of one adapter in degrees Celsius, None when the driver does not say.
    pub fn temperature_of(&self, adapter: i32) -> Option<f32> {
        let get = self.pmlog?;
        let mut data = PmLogData { size: std::mem::size_of::<PmLogData>() as i32, sensors: [[0; 2]; 256] };
        if unsafe { get(self.ctx, adapter, &mut data) } != ADL_OK {
            return None;
        }
        let [supported, value] = data.sensors[PMLOG_TEMPERATURE_EDGE];
        // A value outside what a card can be is a different sensor layout: not shown.
        (supported != 0 && (1..=125).contains(&value)).then_some(value as f32)
    }

    /// The first adapter that reports a temperature.
    pub fn thermal_adapter(&self) -> Option<i32> {
        let mut seen = Vec::new();
        for a in self.adapters() {
            if a.present == 0 || seen.contains(&a.adapter_index) {
                continue;
            }
            seen.push(a.adapter_index);
            if self.temperature_of(a.adapter_index).is_some() {
                return Some(a.adapter_index);
            }
        }
        None
    }

    /// Connected, mapped displays of one ADL adapter: their display indexes.
    fn display_indexes(&self, adapter: i32) -> Vec<i32> {
        unsafe {
            let mut n = 0i32;
            let mut info: *mut DisplayInfo = std::ptr::null_mut();
            let ok = (self.display_info)(self.ctx, adapter, &mut n, &mut info, 0) == ADL_OK;
            let mut out = Vec::new();
            if ok && !info.is_null() && n > 0 && n <= 64 {
                for d in std::slice::from_raw_parts(info, n as usize) {
                    let wanted = ADL_DISPLAY_CONNECTED | ADL_DISPLAY_MAPPED;
                    if d.info_value & wanted == wanted && d.id.logical_adapter_index == adapter {
                        out.push(d.id.logical_index);
                    }
                }
            }
            heap_free(info as *mut c_void);
            out
        }
    }

    /// (current, default, min, max) of the saturation control.
    fn saturation(&self, adapter: i32, display: i32) -> Option<(i32, i32, i32, i32)> {
        let (mut cur, mut def, mut min, mut max, mut step) = (0, 0, 0, 0, 0);
        let r = unsafe {
            (self.color_get)(
                self.ctx,
                adapter,
                display,
                ADL_DISPLAY_COLOR_SATURATION,
                &mut cur,
                &mut def,
                &mut min,
                &mut max,
                &mut step,
            )
        };
        (r == ADL_OK).then_some((cur, def, min, max))
    }

    /// One saturation backend per GDI device name (`\\.\DISPLAY1`) AMD drives.
    pub fn displays(lib: &Arc<AdlLib>) -> Vec<(String, AmdDisplay)> {
        let mut out: Vec<(String, AmdDisplay)> = Vec::new();
        for a in lib.adapters() {
            let device = c_text(&a.display_name);
            if device.is_empty() || out.iter().any(|(d, _)| d.eq_ignore_ascii_case(&device)) {
                continue;
            }
            for display in lib.display_indexes(a.adapter_index) {
                let Some((cur, def, min, max)) = lib.saturation(a.adapter_index, display) else { continue };
                let Some(range) = VibranceRange::amd(min, max, def) else { continue };
                out.push((
                    device.clone(),
                    AmdDisplay { lib: lib.clone(), adapter: a.adapter_index, display, range, original: cur },
                ));
                break;
            }
        }
        out
    }
}

/// One AMD display's saturation control.
pub struct AmdDisplay {
    lib: Arc<AdlLib>,
    adapter: i32,
    display: i32,
    range: VibranceRange,
    original: i32,
}

impl VibranceBackend for AmdDisplay {
    fn name(&self) -> &'static str {
        "AMD saturation"
    }

    fn range(&self) -> VibranceRange {
        self.range
    }

    fn get(&mut self) -> Option<i32> {
        self.lib.saturation(self.adapter, self.display).map(|(cur, ..)| cur)
    }

    fn set(&mut self, level: i32) -> bool {
        let level = level.clamp(self.range.min, self.range.max);
        unsafe {
            (self.lib.color_set)(self.lib.ctx, self.adapter, self.display, ADL_DISPLAY_COLOR_SATURATION, level)
                == ADL_OK
        }
    }

    fn original(&self) -> i32 {
        self.original
    }

    fn set_original(&mut self, level: i32) {
        self.original = level.clamp(self.range.min, self.range.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adl_struct_sizes_match_the_sdk() {
        assert_eq!(std::mem::size_of::<AdapterInfo>(), 1572);
        assert_eq!(std::mem::size_of::<DisplayInfo>(), 552);
    }
}
