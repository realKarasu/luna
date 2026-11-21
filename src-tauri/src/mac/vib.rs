//! macOS gives programs no way to change a graphics driver's vibrance, so every screen keeps
//! the gamma curve only.

use crate::screens::MonitorInfo;
use crate::vibrance::{Vendor, VibranceBackend};
use std::collections::HashMap;

#[derive(Default)]
pub struct Drivers;

impl Drivers {
    pub fn new() -> Drivers {
        Drivers
    }

    pub fn find(&mut self, monitors: &mut [MonitorInfo], _reload: bool) -> HashMap<String, Box<dyn VibranceBackend>> {
        for m in monitors.iter_mut() {
            m.gpu = String::new();
            m.vendor = Vendor::Generic;
        }
        HashMap::new()
    }

    pub fn release(&mut self) {}
}
