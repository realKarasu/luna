//! Borderless windowed changes other programs' windows through Windows calls. Elsewhere it
//! offers nothing (the page only appears on Windows).

use crate::fit::WindowInfo;
use crate::screens::MonitorInfo;
use crate::settings::{Borderless, Fit};

const NOT_HERE: &str = "Borderless windows only exist on Windows.";

pub fn list() -> Vec<WindowInfo> {
    Vec::new()
}

pub fn apply(_id: i64, _fit: &Fit, _monitors: &[MonitorInfo]) -> Result<(), String> {
    Err(NOT_HERE.into())
}

pub fn restore(_id: i64) -> Result<(), String> {
    Err(NOT_HERE.into())
}

pub fn toggle_front(_b: &Borderless, _monitors: &[MonitorInfo]) -> Result<(), String> {
    Err(NOT_HERE.into())
}

pub fn watch(_current: impl Fn() -> Option<(Borderless, Vec<MonitorInfo>)> + Send + 'static) {}
