//! The FPS counter reads Windows event tracing. Elsewhere it stays off.

use crate::games::FpsReading;

pub fn set_enabled(_on: bool, _graph: bool) {}

pub fn stop() {}

pub fn poll() -> Option<FpsReading> {
    None
}
