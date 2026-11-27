//! What the PC is doing, for the lines under the FPS counter: processor and memory from
//! Windows, graphics card load and video memory from the same counters Task Manager reads,
//! and the card's temperature from its driver (AMD or NVIDIA). Nothing here needs
//! administrator. The processor temperature needs a kernel driver and is not read.

use super::adl::AdlLib;
use super::nvapi::NvThermo;
use crate::games::SysReading;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

const PDH_MORE_DATA: u32 = 0x8000_07D2;
const GB: f64 = 1024.0 * 1024.0 * 1024.0;

static WANT: AtomicBool = AtomicBool::new(false);

/// The details are on in the settings. Reading starts and stops in `Sampler::sample`.
pub fn set_enabled(on: bool) {
    WANT.store(on, Ordering::Relaxed);
}

fn ticks(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

/// (idle, busy and idle) processor time since Windows started.
fn cpu_times() -> Option<(u64, u64)> {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    // Kernel time includes idle time.
    Some((ticks(idle), ticks(kernel) + ticks(user)))
}

fn ram_used_gb() -> Option<f32> {
    let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
    Some((m.ullTotalPhys.saturating_sub(m.ullAvailPhys) as f64 / GB) as f32)
}

/// The graphics counters of Windows 10 1709 and later.
struct Pdh {
    query: PDH_HQUERY,
    engines: Option<PDH_HCOUNTER>,
    memory: Option<PDH_HCOUNTER>,
}

impl Pdh {
    fn open() -> Option<Pdh> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let add = |path: PCWSTR| {
                let mut c = PDH_HCOUNTER::default();
                (PdhAddEnglishCounterW(query, path, 0, &mut c) == 0).then_some(c)
            };
            let engines = add(w!("\\GPU Engine(*)\\Utilization Percentage"));
            let memory = add(w!("\\GPU Adapter Memory(*)\\Dedicated Usage"));
            let pdh = Pdh { query, engines, memory };
            if pdh.engines.is_none() && pdh.memory.is_none() {
                return None;
            }
            // Loads are measured between two readings: this is the first.
            let _ = PdhCollectQueryData(query);
            Some(pdh)
        }
    }

    /// Every instance of a counter with its value.
    fn values(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
        unsafe {
            let (mut bytes, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None) != PDH_MORE_DATA
                || bytes == 0
            {
                return Vec::new();
            }
            // The names are stored after the items, in the same buffer.
            let mut buf = vec![0u64; (bytes as usize).div_ceil(8)];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, Some(items)) != 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(items, count as usize)
                .iter()
                .filter(|i| i.FmtValue.CStatus == 0 && !i.szName.is_null())
                .map(|i| (i.szName.to_string().unwrap_or_default(), i.FmtValue.Anonymous.doubleValue))
                .collect()
        }
    }

    /// (graphics card load in percent, video memory in use in GB).
    fn read(&self) -> (Option<f32>, Option<f32>) {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return (None, None);
            }
        }
        let load = self.engines.map(Pdh::values).filter(|v| !v.is_empty()).map(|v| busiest_engine(&v));
        let vram = self
            .memory
            .map(Pdh::values)
            .and_then(|v| v.into_iter().map(|(_, bytes)| bytes).reduce(f64::max))
            .map(|bytes| (bytes / GB) as f32);
        (load, vram)
    }
}

impl Drop for Pdh {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.query);
        }
    }
}

/// Windows counts each engine of the card (3D, video decode, copy) per program, under a
/// name like `pid_1234_luid_0x0_0x1_phys_0_eng_0_engtype_3D`. The load of the card is its
/// busiest engine, every program added up, as Task Manager shows it.
pub fn busiest_engine(values: &[(String, f64)]) -> f32 {
    let mut engines: HashMap<&str, f64> = HashMap::new();
    for (name, v) in values {
        let engine = name.find("luid_").map_or(name.as_str(), |i| &name[i..]);
        *engines.entry(engine).or_default() += v;
    }
    engines.values().copied().fold(0.0, f64::max).clamp(0.0, 100.0) as f32
}

enum Thermo {
    Amd(Arc<AdlLib>, i32),
    Nvidia(NvThermo),
}

impl Thermo {
    fn find() -> Option<Thermo> {
        if let Some(lib) = AdlLib::load() {
            if let Some(adapter) = lib.thermal_adapter() {
                return Some(Thermo::Amd(lib, adapter));
            }
        }
        NvThermo::load().map(Thermo::Nvidia)
    }

    fn read(&self) -> Option<f32> {
        match self {
            Thermo::Amd(lib, adapter) => lib.temperature_of(*adapter),
            Thermo::Nvidia(nv) => nv.read(),
        }
    }
}

/// Lives in the thread that reads the FPS counter.
#[derive(Default)]
pub struct Sampler {
    on: bool,
    cpu: Option<(u64, u64)>,
    pdh: Option<Pdh>,
    thermo: Option<Thermo>,
}

impl Sampler {
    /// What the PC is doing since the last call, None while the details are off.
    pub fn sample(&mut self) -> Option<SysReading> {
        if !WANT.load(Ordering::Relaxed) {
            *self = Sampler::default();
            return None;
        }
        if !self.on {
            self.on = true;
            self.cpu = cpu_times();
            self.pdh = Pdh::open();
            self.thermo = Thermo::find();
            return Some(SysReading { ram: ram_used_gb(), ..Default::default() });
        }
        let now = cpu_times();
        let cpu = match (self.cpu, now) {
            (Some((idle0, all0)), Some((idle1, all1))) if all1 > all0 => {
                let idle = idle1.saturating_sub(idle0) as f64 / (all1 - all0) as f64;
                Some(((1.0 - idle) * 100.0).clamp(0.0, 100.0) as f32)
            }
            _ => None,
        };
        self.cpu = now;
        let (gpu, vram) = self.pdh.as_ref().map_or((None, None), Pdh::read);
        let gpu_temp = self.thermo.as_ref().and_then(Thermo::read);
        Some(SysReading { cpu, gpu, gpu_temp, vram, ram: ram_used_gb() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_as_busy_as_its_busiest_engine() {
        let values = vec![
            ("pid_10_luid_0x0_0x1_phys_0_eng_0_engtype_3D".to_string(), 55.0),
            ("pid_22_luid_0x0_0x1_phys_0_eng_0_engtype_3D".to_string(), 30.5),
            ("pid_22_luid_0x0_0x1_phys_0_eng_3_engtype_VideoDecode".to_string(), 12.0),
            ("pid_10_luid_0x0_0x2_phys_0_eng_0_engtype_3D".to_string(), 4.0),
        ];
        assert!((busiest_engine(&values) - 85.5).abs() < 0.01);
        assert_eq!(busiest_engine(&[]), 0.0);
    }

    /// Reads this PC for real: `cargo test sysmon -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn reads_this_pc() {
        set_enabled(true);
        let mut s = Sampler::default();
        println!("{:?}", s.sample());
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("{:?}", s.sample());
        }
    }
}
