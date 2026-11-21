//! Driver vibrance on Windows: NVAPI on NVIDIA, ADL on AMD, matched to the
//! monitors through the graphics card DXGI reports for each of them.

use super::adl::AdlLib;
use super::gpu;
use super::nvapi::NvLib;
use crate::screens::MonitorInfo;
use crate::vibrance::{Vendor, VibranceBackend};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct Drivers {
    nv: Option<NvLib>,
    adl: Option<Arc<AdlLib>>,
}

impl Drivers {
    pub fn new() -> Drivers {
        Drivers::default()
    }

    /// Fills in each monitor's graphics card and returns the driver vibrance found per
    /// monitor id. `reload` starts ADL afresh, as its adapter list can change when screens
    /// come and go.
    pub fn find(&mut self, monitors: &mut [MonitorInfo], reload: bool) -> HashMap<String, Box<dyn VibranceBackend>> {
        if reload {
            self.adl = None;
        }
        let gpus = gpu::detect();
        for m in monitors.iter_mut() {
            match gpus.get(&gpu::key(&m.id)) {
                Some(g) => {
                    m.gpu = g.name.clone();
                    m.vendor = g.vendor;
                }
                None => {
                    m.gpu = String::new();
                    m.vendor = Vendor::Generic;
                }
            }
        }
        let monitors: &[MonitorInfo] = monitors;
        // When DXGI tells nothing, a driver that names the monitor is trusted.
        let wants = |m: &MonitorInfo, v: Vendor| gpus.is_empty() || m.vendor == v;
        let any = |v: Vendor| gpus.is_empty() || monitors.iter().any(|m| m.vendor == v);
        let mut found: HashMap<String, Box<dyn VibranceBackend>> = HashMap::new();

        if any(Vendor::Nvidia) {
            if self.nv.is_none() {
                self.nv = NvLib::load();
            }
            if let Some(nv) = &self.nv {
                let mut unnamed = Vec::new();
                for (name, d) in nv.displays() {
                    let target = name.and_then(|n| {
                        monitors.iter().find(|m| m.id.eq_ignore_ascii_case(&n) && wants(m, Vendor::Nvidia))
                    });
                    match target {
                        Some(m) if !found.contains_key(&m.id) => {
                            found.insert(m.id.clone(), Box::new(d));
                        }
                        Some(_) => {}
                        None => unnamed.push(d),
                    }
                }
                // Older drivers do not name their displays: hand them out in order.
                let mut rest = unnamed.into_iter();
                for m in monitors.iter().filter(|m| m.vendor == Vendor::Nvidia) {
                    if found.contains_key(&m.id) {
                        continue;
                    }
                    match rest.next() {
                        Some(d) => {
                            found.insert(m.id.clone(), Box::new(d));
                        }
                        None => break,
                    }
                }
            }
        }

        if any(Vendor::Amd) {
            if self.adl.is_none() {
                self.adl = AdlLib::load();
            }
            if let Some(lib) = &self.adl {
                for (device, d) in AdlLib::displays(lib) {
                    if let Some(m) = monitors.iter().find(|m| m.id.eq_ignore_ascii_case(&device) && wants(m, Vendor::Amd))
                    {
                        found.entry(m.id.clone()).or_insert_with(|| Box::new(d));
                    }
                }
            }
        }
        found
    }

    /// Nothing uses the drivers any more: ADL2_Main_Control_Destroy through AdlLib's drop.
    pub fn release(&mut self) {
        self.adl = None;
    }
}
