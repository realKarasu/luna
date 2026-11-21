//! Game profiles watch programs and change screen modes through Windows calls. Elsewhere
//! there is nothing to watch (the Games page only appears on Windows).

use crate::settings::DisplayMode;
use std::collections::HashSet;

pub fn running() -> HashSet<String> {
    HashSet::new()
}

pub fn modes(_device: &str) -> Vec<DisplayMode> {
    Vec::new()
}

pub fn set_mode(_device: &str, _mode: &DisplayMode) -> Result<(), String> {
    Err("Screen modes per game only exist on Windows.".into())
}

pub fn restore_mode(_device: &str) {}

pub fn reset_mode(_device: &str) {}

pub fn restore_all_modes() {}
