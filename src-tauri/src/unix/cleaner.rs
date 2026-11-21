//! The Clean page uses Windows folders and calls. Elsewhere it finds nothing (the page only
//! appears on Windows).

use crate::sweep::{CleanReport, CleanResult};

pub fn scan() -> CleanReport {
    CleanReport::default()
}

pub fn run(_ids: &[String]) -> CleanResult {
    CleanResult::default()
}
