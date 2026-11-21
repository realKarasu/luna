//! Potato Mode changes Windows settings and program priorities. Elsewhere it does nothing.

use crate::games::PotatoStatus;

pub fn status() -> PotatoStatus {
    PotatoStatus::default()
}

pub fn active() -> bool {
    false
}

pub fn set_manual(_on: bool, _keep: &[String]) -> Result<PotatoStatus, String> {
    Err("Potato Mode only exists on Windows.".into())
}

pub fn set_game(_exe: Option<&str>, _keep: &[String]) {}

pub fn refresh(_keep: &[String]) {}

pub fn stop() {}

pub fn stop_after_panic() {}

pub fn recover() {}
