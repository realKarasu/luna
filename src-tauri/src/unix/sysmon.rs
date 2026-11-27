//! The system numbers under the FPS counter are read from Windows. Elsewhere there are none.

use crate::games::SysReading;

pub fn set_enabled(_on: bool) {}

#[derive(Default)]
pub struct Sampler;

impl Sampler {
    pub fn sample(&mut self) -> Option<SysReading> {
        None
    }
}
