//! Vendor detection and the vibrance maths shared by every GPU backend.
//! Pure code (no Windows calls), so it is unit tested on any machine.
//!
//! Luna's vibrance runs from 0 to 250 %, with 100 % meaning "no change". Each
//! driver has its own integer range and default. A backend reports that range
//! together with the Luna percentages its minimum and maximum stand for; the
//! engine maps the slider onto it, and whatever the driver cannot reach is
//! made up by the full-screen saturation matrix.

use serde::Serialize;

pub const VENDOR_NVIDIA: u32 = 0x10DE;
pub const VENDOR_AMD: u32 = 0x1002;
pub const VENDOR_INTEL: u32 = 0x8086;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Generic,
}

impl Vendor {
    pub fn from_id(vendor_id: u32) -> Vendor {
        match vendor_id {
            VENDOR_NVIDIA => Vendor::Nvidia,
            VENDOR_AMD => Vendor::Amd,
            VENDOR_INTEL => Vendor::Intel,
            _ => Vendor::Generic,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Vendor::Nvidia => "NVIDIA",
            Vendor::Amd => "AMD",
            Vendor::Intel => "Intel",
            Vendor::Generic => "Other",
        }
    }
}

/// A driver's vibrance range and what its ends mean in Luna percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VibranceRange {
    pub min: i32,
    pub max: i32,
    /// The driver's neutral level: Luna 100 %.
    pub default: i32,
    /// Luna percent reached at `min` (0 when the driver can desaturate, 100 when it cannot).
    pub pct_min: f64,
    /// Luna percent reached at `max` (usually 200).
    pub pct_max: f64,
}

impl VibranceRange {
    /// NVIDIA digital vibrance. The driver maximum is treated as 200 %.
    /// Older drivers only expose levels from the default upwards.
    pub fn nvidia(min: i32, max: i32, default: i32) -> Option<VibranceRange> {
        if max <= min {
            return None;
        }
        let default = default.clamp(min, max);
        if default == max {
            return None;
        }
        let pct_min = if default > min { 0.0 } else { 100.0 };
        Some(VibranceRange { min, max, default, pct_min, pct_max: 200.0 })
    }

    /// AMD (ADL) saturation: the value is already a percentage of the default,
    /// so min 0 / default 100 / max 200 stand for 0 %, 100 % and 200 %.
    pub fn amd(min: i32, max: i32, default: i32) -> Option<VibranceRange> {
        if max <= min || default <= 0 || default < min || default >= max {
            return None;
        }
        let pct = |v: i32| 100.0 * v as f64 / default as f64;
        Some(VibranceRange { min, max, default, pct_min: pct(min).clamp(0.0, 100.0), pct_max: pct(max).max(100.0) })
    }

    /// Driver level for a Luna percent. 100 % gives `default`; the percent is
    /// clamped to what the driver covers.
    pub fn level_for(&self, percent: f64) -> i32 {
        let p = if percent.is_finite() { percent } else { 100.0 }.clamp(self.pct_min, self.pct_max);
        let level = if p >= 100.0 {
            let span = self.pct_max - 100.0;
            if span <= 0.0 {
                self.default as f64
            } else {
                self.default as f64 + (self.max - self.default) as f64 * (p - 100.0) / span
            }
        } else {
            let span = 100.0 - self.pct_min;
            if span <= 0.0 {
                self.default as f64
            } else {
                self.min as f64 + (self.default - self.min) as f64 * (p - self.pct_min) / span
            }
        };
        (level.round() as i32).clamp(self.min, self.max)
    }

    /// The Luna percents this driver can show by itself.
    pub fn covers(&self) -> (f64, f64) {
        (self.pct_min, self.pct_max)
    }
}

/// What every vendor backend offers for one display.
pub trait VibranceBackend: Send {
    /// Short name shown in the interface, e.g. "NVIDIA digital vibrance".
    fn name(&self) -> &'static str;
    fn range(&self) -> VibranceRange;
    /// The level the driver reports right now.
    fn get(&mut self) -> Option<i32>;
    /// Writes a level; false when the driver refused.
    fn set(&mut self, level: i32) -> bool;
    /// The level found when Luna took over this display.
    fn original(&self) -> i32;
    /// Makes `level` the one `restore` goes back to (crash recovery).
    fn set_original(&mut self, level: i32);
    /// Puts the original level back.
    fn restore(&mut self) -> bool {
        let o = self.original();
        self.set(o)
    }
}

/// The Luna percents every display's driver can reach. The colour matrix is
/// one full-screen filter, so it has to work the same on every screen: if one
/// screen has no driver vibrance, the matrix does all the work everywhere.
/// `None` means no screen has driver vibrance.
pub fn common_cover(per_display: &[Option<(f64, f64)>]) -> Option<(f64, f64)> {
    if per_display.is_empty() {
        return None;
    }
    let mut low = f64::MIN;
    let mut high = f64::MAX;
    for c in per_display {
        let (l, h) = c.unwrap_or((100.0, 100.0));
        low = low.max(l);
        high = high.min(h);
    }
    if high <= low || (low >= 100.0 && high <= 100.0) {
        None
    } else {
        Some((low, high))
    }
}

/// The part of `vibrance` given to the drivers, inside the common cover.
pub fn driver_percent(vibrance: f64, cover: Option<(f64, f64)>) -> f64 {
    match cover {
        Some((low, high)) => vibrance.clamp(low, high),
        None => 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_ids() {
        assert_eq!(Vendor::from_id(0x10DE), Vendor::Nvidia);
        assert_eq!(Vendor::from_id(0x1002), Vendor::Amd);
        assert_eq!(Vendor::from_id(0x8086), Vendor::Intel);
        assert_eq!(Vendor::from_id(0x1414), Vendor::Generic); // Microsoft Basic Render
        assert_eq!(Vendor::from_id(0), Vendor::Generic);
        assert_eq!(Vendor::Amd.label(), "AMD");
    }

    #[test]
    fn nvidia_ex_range_maps_0_100_200() {
        // NvAPI Ex range: 0..100, default 50.
        let r = VibranceRange::nvidia(0, 100, 50).unwrap();
        assert_eq!(r.level_for(0.0), 0);
        assert_eq!(r.level_for(50.0), 25);
        assert_eq!(r.level_for(100.0), 50);
        assert_eq!(r.level_for(150.0), 75);
        assert_eq!(r.level_for(200.0), 100);
        // Beyond the driver maximum stays at the maximum (the matrix adds the rest).
        assert_eq!(r.level_for(250.0), 100);
        assert_eq!(r.covers(), (0.0, 200.0));
    }

    #[test]
    fn nvidia_legacy_range_starts_at_default() {
        // Legacy DVC: 0..63 with the default at 0, it cannot desaturate.
        let r = VibranceRange::nvidia(0, 63, 0).unwrap();
        assert_eq!(r.covers(), (100.0, 200.0));
        assert_eq!(r.level_for(0.0), 0);
        assert_eq!(r.level_for(100.0), 0);
        assert_eq!(r.level_for(150.0), 32);
        assert_eq!(r.level_for(200.0), 63);
        assert_eq!(r.level_for(250.0), 63);
    }

    #[test]
    fn amd_range_is_a_percentage() {
        let r = VibranceRange::amd(0, 200, 100).unwrap();
        assert_eq!(r.covers(), (0.0, 200.0));
        assert_eq!(r.level_for(0.0), 0);
        assert_eq!(r.level_for(37.0), 37);
        assert_eq!(r.level_for(100.0), 100);
        assert_eq!(r.level_for(175.0), 175);
        assert_eq!(r.level_for(250.0), 200);
    }

    #[test]
    fn amd_odd_ranges() {
        // Max 300 with default 100 covers up to 300 %, so Luna's 250 % fits.
        let r = VibranceRange::amd(0, 300, 100).unwrap();
        assert_eq!(r.covers(), (0.0, 300.0));
        assert_eq!(r.level_for(250.0), 250);
        // A minimum above zero cannot fully desaturate.
        let r = VibranceRange::amd(50, 200, 100).unwrap();
        assert_eq!(r.covers(), (50.0, 200.0));
        assert_eq!(r.level_for(0.0), 50);
        assert_eq!(r.level_for(75.0), 75);
        // Broken reports are refused.
        assert!(VibranceRange::amd(0, 0, 0).is_none());
        assert!(VibranceRange::amd(0, 200, 0).is_none());
        assert!(VibranceRange::amd(0, 200, 200).is_none());
        assert!(VibranceRange::nvidia(10, 10, 10).is_none());
    }

    #[test]
    fn non_finite_percent_is_neutral() {
        let r = VibranceRange::amd(0, 200, 100).unwrap();
        assert_eq!(r.level_for(f64::NAN), 100);
    }

    #[test]
    fn common_cover_is_the_intersection() {
        assert_eq!(common_cover(&[]), None);
        assert_eq!(common_cover(&[Some((0.0, 200.0))]), Some((0.0, 200.0)));
        assert_eq!(common_cover(&[Some((0.0, 200.0)), Some((100.0, 200.0))]), Some((100.0, 200.0)));
        assert_eq!(common_cover(&[Some((0.0, 300.0)), Some((0.0, 200.0))]), Some((0.0, 200.0)));
        // One screen without driver vibrance: the matrix does everything.
        assert_eq!(common_cover(&[Some((0.0, 200.0)), None]), None);
    }

    #[test]
    fn split_between_driver_and_matrix() {
        use crate::color::vibrance_matrix_factor;
        let cover = Some((0.0, 200.0));
        // 250 %: the driver gives 200, the matrix multiplies by 1.25.
        assert_eq!(driver_percent(250.0, cover), 200.0);
        assert!((vibrance_matrix_factor(250.0, cover) - 1.25).abs() < 1e-9);
        // 150 %: all in the driver.
        assert_eq!(driver_percent(150.0, cover), 150.0);
        assert!((vibrance_matrix_factor(150.0, cover) - 1.0).abs() < 1e-9);
        // No driver: neutral driver, the matrix does it all.
        assert_eq!(driver_percent(180.0, None), 100.0);
        assert!((vibrance_matrix_factor(180.0, None) - 1.8).abs() < 1e-9);
        // Legacy NVIDIA cannot desaturate: 40 % comes from the matrix.
        let legacy = Some((100.0, 200.0));
        assert_eq!(driver_percent(40.0, legacy), 100.0);
        assert!((vibrance_matrix_factor(40.0, legacy) - 0.4).abs() < 1e-9);
    }

    struct Fake {
        level: i32,
        original: i32,
    }

    impl VibranceBackend for Fake {
        fn name(&self) -> &'static str {
            "fake"
        }
        fn range(&self) -> VibranceRange {
            VibranceRange::amd(0, 200, 100).unwrap()
        }
        fn get(&mut self) -> Option<i32> {
            Some(self.level)
        }
        fn set(&mut self, level: i32) -> bool {
            self.level = level;
            true
        }
        fn original(&self) -> i32 {
            self.original
        }
        fn set_original(&mut self, level: i32) {
            self.original = level;
        }
    }

    #[test]
    fn restore_goes_back_to_original() {
        let mut b: Box<dyn VibranceBackend> = Box::new(Fake { level: 120, original: 120 });
        let lvl = b.range().level_for(180.0);
        assert!(b.set(lvl));
        assert_eq!(b.get(), Some(180));
        assert!(b.restore());
        assert_eq!(b.get(), Some(120));
    }
}

