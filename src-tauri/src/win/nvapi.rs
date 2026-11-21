//! NVIDIA digital vibrance through NVAPI, loaded at run time from nvapi64.dll.
//! Nothing is linked: when the DLL or a function is missing, Luna simply falls
//! back to the saturation matrix.

use crate::vibrance::{VibranceBackend, VibranceRange};
use std::ffi::c_void;
use windows::core::{s, w};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};

type QueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;
type NvInitialize = unsafe extern "C" fn() -> i32;
type NvEnumDisplay = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type NvGetName = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;
type NvGetDvcEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcInfoEx) -> i32;
type NvSetDvcEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcInfoEx) -> i32;
type NvGetDvc = unsafe extern "C" fn(*mut c_void, u32, *mut DvcInfo) -> i32;
type NvSetDvc = unsafe extern "C" fn(*mut c_void, u32, i32) -> i32;

const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_ENUM_NVIDIA_DISPLAY_HANDLE: u32 = 0x9ABD_D40D;
const ID_GET_ASSOCIATED_NVIDIA_DISPLAY_NAME: u32 = 0x22A7_8B05;
const ID_GET_DVC_INFO_EX: u32 = 0x0E45_002D;
const ID_SET_DVC_LEVEL_EX: u32 = 0x4A82_C2B1;
const ID_GET_DVC_INFO: u32 = 0x4085_DE45;
const ID_SET_DVC_LEVEL: u32 = 0x1724_09B4;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct DvcInfo {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct DvcInfoEx {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
    default: i32,
}

fn version_of<T>() -> u32 {
    std::mem::size_of::<T>() as u32 | (1 << 16)
}

#[derive(Clone, Copy)]
struct Fns {
    get_ex: Option<NvGetDvcEx>,
    set_ex: Option<NvSetDvcEx>,
    get: Option<NvGetDvc>,
    set: Option<NvSetDvc>,
}

/// The loaded NVAPI functions. The DLL stays loaded for the life of Luna.
pub struct NvLib {
    fns: Fns,
    enum_disp: NvEnumDisplay,
    display_name: Option<NvGetName>,
}

// Plain function pointers into a DLL that is never unloaded.
unsafe impl Send for NvLib {}

impl NvLib {
    /// Loads and initialises NVAPI. None on any PC without a working NVIDIA driver.
    pub fn load() -> Option<NvLib> {
        unsafe {
            // Only the copy in System32, never a file placed next to Luna.
            let lib = LoadLibraryExW(w!("nvapi64.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32).ok()?;
            let qi_ptr = GetProcAddress(lib, s!("nvapi_QueryInterface"))?;
            let qi: QueryInterface = std::mem::transmute(qi_ptr);
            let get = |id: u32| {
                let p = qi(id);
                if p.is_null() {
                    None
                } else {
                    Some(p)
                }
            };
            let init: NvInitialize = std::mem::transmute(get(ID_INITIALIZE)?);
            if init() != 0 {
                return None;
            }
            let enum_disp: NvEnumDisplay = std::mem::transmute(get(ID_ENUM_NVIDIA_DISPLAY_HANDLE)?);
            let display_name: Option<NvGetName> =
                get(ID_GET_ASSOCIATED_NVIDIA_DISPLAY_NAME).map(|p| std::mem::transmute(p));
            let fns = Fns {
                get_ex: get(ID_GET_DVC_INFO_EX).map(|p| std::mem::transmute(p)),
                set_ex: get(ID_SET_DVC_LEVEL_EX).map(|p| std::mem::transmute(p)),
                get: get(ID_GET_DVC_INFO).map(|p| std::mem::transmute(p)),
                set: get(ID_SET_DVC_LEVEL).map(|p| std::mem::transmute(p)),
            };
            if fns.set_ex.is_none() && fns.set.is_none() {
                return None;
            }
            Some(NvLib { fns, enum_disp, display_name })
        }
    }

    /// Every NVIDIA display whose vibrance can be read and written, with the
    /// GDI device name the driver reports for it (when it does).
    pub fn displays(&self) -> Vec<(Option<String>, NvDisplay)> {
        let mut out = Vec::new();
        for i in 0..16u32 {
            let mut h: *mut c_void = std::ptr::null_mut();
            if unsafe { (self.enum_disp)(i, &mut h) } != 0 || h.is_null() {
                break;
            }
            let name = self.display_name.and_then(|f| {
                let mut buf = [0u8; 64];
                if unsafe { f(h, buf.as_mut_ptr()) } != 0 {
                    return None;
                }
                let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                let s = String::from_utf8_lossy(&buf[..len]).trim().to_string();
                (!s.is_empty()).then_some(s)
            });
            if let Some(d) = NvDisplay::open(self.fns, h) {
                out.push((name, d));
            }
        }
        out
    }
}

/// One NVIDIA display's digital vibrance.
pub struct NvDisplay {
    fns: Fns,
    handle: *mut c_void,
    ex: bool,
    range: VibranceRange,
    original: i32,
}

// The handle is an opaque NVAPI token, only used behind the engine mutex.
unsafe impl Send for NvDisplay {}

impl NvDisplay {
    fn open(fns: Fns, h: *mut c_void) -> Option<NvDisplay> {
        unsafe {
            if let (Some(g), Some(_)) = (fns.get_ex, fns.set_ex) {
                let mut info = DvcInfoEx { version: version_of::<DvcInfoEx>(), ..Default::default() };
                if g(h, 0, &mut info) == 0 {
                    if let Some(range) = VibranceRange::nvidia(info.min, info.max, info.default) {
                        return Some(NvDisplay { fns, handle: h, ex: true, range, original: info.current });
                    }
                }
            }
            if let (Some(g), Some(_)) = (fns.get, fns.set) {
                let mut info = DvcInfo { version: version_of::<DvcInfo>(), ..Default::default() };
                // The old call has no default: the driver's neutral level is its minimum.
                if g(h, 0, &mut info) == 0 {
                    if let Some(range) = VibranceRange::nvidia(info.min, info.max, info.min) {
                        return Some(NvDisplay { fns, handle: h, ex: false, range, original: info.current });
                    }
                }
            }
            None
        }
    }
}

impl VibranceBackend for NvDisplay {
    fn name(&self) -> &'static str {
        "NVIDIA digital vibrance"
    }

    fn range(&self) -> VibranceRange {
        self.range
    }

    fn get(&mut self) -> Option<i32> {
        unsafe {
            if self.ex {
                let g = self.fns.get_ex?;
                let mut info = DvcInfoEx { version: version_of::<DvcInfoEx>(), ..Default::default() };
                (g(self.handle, 0, &mut info) == 0).then_some(info.current)
            } else {
                let g = self.fns.get?;
                let mut info = DvcInfo { version: version_of::<DvcInfo>(), ..Default::default() };
                (g(self.handle, 0, &mut info) == 0).then_some(info.current)
            }
        }
    }

    fn set(&mut self, level: i32) -> bool {
        let level = level.clamp(self.range.min, self.range.max);
        unsafe {
            if self.ex {
                let (Some(get), Some(set)) = (self.fns.get_ex, self.fns.set_ex) else { return false };
                let mut info = DvcInfoEx { version: version_of::<DvcInfoEx>(), ..Default::default() };
                if get(self.handle, 0, &mut info) != 0 {
                    return false;
                }
                info.current = level;
                set(self.handle, 0, &mut info) == 0
            } else if let Some(set) = self.fns.set {
                set(self.handle, 0, level) == 0
            } else {
                false
            }
        }
    }

    fn original(&self) -> i32 {
        self.original
    }

    fn set_original(&mut self, level: i32) {
        self.original = level.clamp(self.range.min, self.range.max);
    }
}
