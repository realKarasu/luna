//! What every platform knows about a screen, and the parts of gamma handling
//! that do not depend on the system.

use crate::gamma::{self, Ramp};
use crate::vibrance::Vendor;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    /// The system's name for the screen: `\\.\DISPLAY1` on Windows, `display-1` on macOS,
    /// the RandR output name such as `DP-1` on Linux.
    pub id: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
    /// The graphics card driving this monitor, e.g. "AMD Radeon RX 7800 XT".
    pub gpu: String,
    /// "nvidia", "amd", "intel" or "generic".
    pub vendor: Vendor,
    /// What gives this monitor its vibrance, e.g. "AMD saturation" or "Color matrix".
    pub vibrance: String,
}

pub const MATRIX_VIBRANCE: &str = "Color matrix";

impl MonitorInfo {
    /// Same place and size: GPU details are filled in later and do not count.
    pub fn same_layout(a: &[MonitorInfo], b: &[MonitorInfo]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(x, y)| {
                x.id == y.id
                    && x.x == y.x
                    && x.y == y.y
                    && x.width == y.width
                    && x.height == y.height
                    && x.primary == y.primary
            })
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }

    fn distance2(&self, x: i32, y: i32) -> i64 {
        let dx = (self.x - x).max(0).max(x - (self.x + self.width - 1)) as i64;
        let dy = (self.y - y).max(0).max(y - (self.y + self.height - 1)) as i64;
        dx * dx + dy * dy
    }
}

/// The monitor that contains the point, or the nearest one.
#[allow(dead_code)]
pub fn monitor_in(list: Vec<MonitorInfo>, x: i32, y: i32) -> Option<MonitorInfo> {
    if let Some(i) = list.iter().position(|m| m.contains(x, y)) {
        return list.into_iter().nth(i);
    }
    list.into_iter().min_by_key(|m| m.distance2(x, y))
}

/// Sorts screens the way Luna shows them: primary first, then left to right.
pub fn sort(list: &mut [MonitorInfo]) {
    list.sort_by_key(|m| (!m.primary, m.x, m.y));
}

/// Sets `target` through `set`; if the system refuses it (Windows limits how far a ramp may
/// stray from identity unless the machine-wide gamma range was unlocked), finds the strongest
/// mix between `base` and `target` that is accepted.
/// Returns the fraction of the effect that was applied (0..1), or None on failure.
pub fn best_effort(set: impl Fn(&Ramp) -> bool, base: &Ramp, target: &Ramp) -> Option<f64> {
    if set(target) {
        return Some(1.0);
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    let mut best: Option<f64> = None;
    for _ in 0..7 {
        let mid = (lo + hi) / 2.0;
        if set(&gamma::blend_ramp(base, target, mid)) {
            best = Some(mid);
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if let Some(t) = best {
        // Leave the last accepted mix in place.
        let _ = set(&gamma::blend_ramp(base, target, t));
    }
    best
}

/// Resamples one channel of any length to `n` entries (linear).
#[allow(dead_code)]
pub fn resample(src: &[f64], n: usize) -> Vec<f64> {
    if src.is_empty() || n == 0 {
        return vec![0.0; n];
    }
    if src.len() == 1 || n == 1 {
        return vec![src[0]; n];
    }
    (0..n)
        .map(|i| {
            let pos = i as f64 * (src.len() - 1) as f64 / (n - 1) as f64;
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(src.len() - 1);
            let f = pos - lo as f64;
            src[lo] * (1.0 - f) + src[hi] * f
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(id: &str, x: i32, y: i32) -> MonitorInfo {
        MonitorInfo {
            id: id.into(),
            label: id.into(),
            x,
            y,
            width: 1920,
            height: 1080,
            primary: x == 0,
            gpu: String::new(),
            vendor: Vendor::Generic,
            vibrance: MATRIX_VIBRANCE.into(),
        }
    }

    #[test]
    fn finds_monitor_at_point() {
        let list = vec![mon("a", 0, 0), mon("b", 1920, 0)];
        assert_eq!(monitor_in(list.clone(), 100, 100).unwrap().id, "a");
        assert_eq!(monitor_in(list.clone(), 2000, 500).unwrap().id, "b");
        assert_eq!(monitor_in(list.clone(), 5000, 500).unwrap().id, "b");
        assert_eq!(monitor_in(list, -50, 2000).unwrap().id, "a");
        assert!(monitor_in(vec![], 0, 0).is_none());
    }

    #[test]
    fn resample_keeps_ends() {
        let src: Vec<f64> = (0..1024).map(|i| i as f64 / 1023.0).collect();
        let r = resample(&src, 256);
        assert_eq!(r.len(), 256);
        assert!((r[0] - 0.0).abs() < 1e-9 && (r[255] - 1.0).abs() < 1e-9);
        assert!((r[128] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(resample(&[0.5], 3), vec![0.5; 3]);
    }

    #[test]
    fn best_effort_finds_a_mix() {
        let base = gamma::identity_ramp();
        let target = gamma::build_ramp(&gamma::Tone { gamma: 3.0, ..gamma::Tone::default() });
        assert_eq!(best_effort(|_| true, &base, &target), Some(1.0));
        assert_eq!(best_effort(|_| false, &base, &target), None);
        let limit = |r: &Ramp| {
            let d = (0..256).map(|i| r[0][i].abs_diff(base[0][i])).max().unwrap_or(0);
            d < 9000
        };
        let f = best_effort(limit, &base, &target).unwrap();
        assert!(f > 0.0 && f < 1.0);
    }
}
