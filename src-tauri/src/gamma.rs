//! Gamma ramp math. Pure code, no Windows calls, fully unit tested.
//!
//! The frontend has a line-for-line port of `channel_value` in `src/lib/math.ts`
//! so the before/after preview uses exactly the same curve as the real output.

use serde::{Deserialize, Serialize};

pub const RAMP_SIZE: usize = 256;

/// Three channels (red, green, blue) of 256 16-bit entries, the layout
/// `SetDeviceGammaRamp` expects (`WORD[3][256]`).
pub type Ramp = [[u16; RAMP_SIZE]; 3];

pub const GAMMA_MIN: f64 = 0.60;
pub const GAMMA_MAX: f64 = 3.00;
pub const BRIGHTNESS_MIN: f64 = -20.0;
pub const BRIGHTNESS_MAX: f64 = 20.0;
pub const CONTRAST_MIN: f64 = 70.0;
pub const CONTRAST_MAX: f64 = 130.0;
pub const CHANNEL_MIN: f64 = 20.0;
pub const CHANNEL_MAX: f64 = 180.0;

/// Tone settings for one monitor.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tone {
    /// 0.60 to 3.00, 1.00 is neutral. Values above 1 lift the shadows.
    pub gamma: f64,
    /// -20 to +20, 0 is neutral. Each step shifts the output by 0.5 %.
    pub brightness: f64,
    /// 70 to 130 %, 100 is neutral.
    pub contrast: f64,
    /// 20 to 180 %, 100 is neutral.
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

impl Default for Tone {
    fn default() -> Self {
        Tone { gamma: 1.0, brightness: 0.0, contrast: 100.0, red: 100.0, green: 100.0, blue: 100.0 }
    }
}

fn finite_or(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

impl Tone {
    /// The same tone with every value forced into its allowed range.
    pub fn clamped(&self) -> Tone {
        Tone {
            gamma: finite_or(self.gamma, 1.0).clamp(GAMMA_MIN, GAMMA_MAX),
            brightness: finite_or(self.brightness, 0.0).clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX),
            contrast: finite_or(self.contrast, 100.0).clamp(CONTRAST_MIN, CONTRAST_MAX),
            red: finite_or(self.red, 100.0).clamp(CHANNEL_MIN, CHANNEL_MAX),
            green: finite_or(self.green, 100.0).clamp(CHANNEL_MIN, CHANNEL_MAX),
            blue: finite_or(self.blue, 100.0).clamp(CHANNEL_MIN, CHANNEL_MAX),
        }
    }

    pub fn is_neutral(&self) -> bool {
        let t = self.clamped();
        (t.gamma - 1.0).abs() < 1e-6
            && t.brightness.abs() < 1e-6
            && (t.contrast - 100.0).abs() < 1e-6
            && (t.red - 100.0).abs() < 1e-6
            && (t.green - 100.0).abs() < 1e-6
            && (t.blue - 100.0).abs() < 1e-6
    }

    fn channel_gain(&self, ch: usize) -> f64 {
        match ch {
            0 => self.red,
            1 => self.green,
            _ => self.blue,
        }
    }
}

/// Output level (0..1) for an input level `x` (0..1) on channel `ch` (0 red, 1 green, 2 blue).
/// Order: gamma curve, contrast around mid grey, brightness offset, channel gain, clamp.
/// `t` must already be clamped.
pub fn channel_value(x: f64, t: &Tone, ch: usize) -> f64 {
    let x = x.clamp(0.0, 1.0);
    let mut v = x.powf(1.0 / t.gamma);
    v = (v - 0.5) * (t.contrast / 100.0) + 0.5;
    v += t.brightness / 200.0;
    v *= t.channel_gain(ch) / 100.0;
    v.clamp(0.0, 1.0)
}

/// Builds the full ramp for a tone.
pub fn build_ramp(tone: &Tone) -> Ramp {
    let t = tone.clamped();
    let mut ramp = [[0u16; RAMP_SIZE]; 3];
    for (ch, channel) in ramp.iter_mut().enumerate() {
        for (i, entry) in channel.iter_mut().enumerate() {
            let x = i as f64 / (RAMP_SIZE - 1) as f64;
            *entry = (channel_value(x, &t, ch) * 65535.0).round() as u16;
        }
    }
    ramp
}

/// The ramp Windows uses when nothing is changed: entry i = i * 257.
pub fn identity_ramp() -> Ramp {
    let mut ramp = [[0u16; RAMP_SIZE]; 3];
    for channel in ramp.iter_mut() {
        for (i, entry) in channel.iter_mut().enumerate() {
            *entry = (i as u16) * 257;
        }
    }
    ramp
}

/// Linear mix of two ramps: `t = 0` gives `a`, `t = 1` gives `b`.
pub fn blend_ramp(a: &Ramp, b: &Ramp, t: f64) -> Ramp {
    let t = t.clamp(0.0, 1.0);
    let mut out = [[0u16; RAMP_SIZE]; 3];
    for ch in 0..3 {
        for i in 0..RAMP_SIZE {
            let v = a[ch][i] as f64 + (b[ch][i] as f64 - a[ch][i] as f64) * t;
            out[ch][i] = v.round().clamp(0.0, 65535.0) as u16;
        }
    }
    out
}

/// True when no entry decreases from one step to the next.
pub fn is_monotonic(r: &Ramp) -> bool {
    r.iter().all(|c| c.windows(2).all(|w| w[0] <= w[1]))
}

/// Largest distance of any entry from the identity ramp.
pub fn max_deviation(r: &Ramp) -> u16 {
    let id = identity_ramp();
    let mut m = 0u16;
    for ch in 0..3 {
        for i in 0..RAMP_SIZE {
            m = m.max(r[ch][i].abs_diff(id[ch][i]));
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_tone_is_identity() {
        let r = build_ramp(&Tone::default());
        assert_eq!(r, identity_ramp());
        assert!(Tone::default().is_neutral());
    }

    #[test]
    fn endpoints_of_neutral_ramp() {
        let r = build_ramp(&Tone::default());
        for ch in 0..3 {
            assert_eq!(r[ch][0], 0);
            assert_eq!(r[ch][255], 65535);
        }
    }

    #[test]
    fn gamma_above_one_lifts_midtones() {
        let t = Tone { gamma: 2.0, ..Tone::default() };
        let r = build_ramp(&t);
        let id = identity_ramp();
        assert!(r[1][64] > id[1][64]);
        assert!(r[1][128] > id[1][128]);
        // Black and white stay put.
        assert_eq!(r[1][0], 0);
        assert_eq!(r[1][255], 65535);
        // x^(1/2) at x = 0.25 is 0.5
        let expected = ((64.0f64 / 255.0).powf(0.5) * 65535.0).round() as u16;
        assert_eq!(r[1][64], expected);
    }

    #[test]
    fn gamma_below_one_darkens_midtones() {
        let t = Tone { gamma: 0.6, ..Tone::default() };
        let r = build_ramp(&t);
        assert!(r[0][128] < identity_ramp()[0][128]);
    }

    #[test]
    fn ramps_are_monotonic_across_ranges() {
        for g in [0.6, 1.0, 1.7, 3.0] {
            for b in [-20.0, 0.0, 20.0] {
                for c in [70.0, 100.0, 130.0] {
                    let t = Tone { gamma: g, brightness: b, contrast: c, red: 180.0, green: 20.0, blue: 100.0 };
                    assert!(is_monotonic(&build_ramp(&t)), "g={g} b={b} c={c}");
                }
            }
        }
    }

    #[test]
    fn channel_gain_scales_one_channel() {
        let t = Tone { red: 50.0, ..Tone::default() };
        let r = build_ramp(&t);
        assert_eq!(r[0][255], 32768); // 0.5 * 65535 rounded
        assert_eq!(r[1][255], 65535);
        assert_eq!(r[2][200], identity_ramp()[2][200]);
    }

    #[test]
    fn brightness_offsets_output() {
        let t = Tone { brightness: 20.0, ..Tone::default() };
        let r = build_ramp(&t);
        assert_eq!(r[0][0], (0.1f64 * 65535.0).round() as u16);
        let t = Tone { brightness: -20.0, ..Tone::default() };
        assert_eq!(build_ramp(&t)[0][0], 0);
    }

    #[test]
    fn contrast_pivots_around_mid_grey() {
        let t = Tone { contrast: 130.0, ..Tone::default() };
        let v = channel_value(0.5, &t.clamped(), 0);
        assert!((v - 0.5).abs() < 1e-12);
        assert!(channel_value(0.75, &t, 0) > 0.75);
        assert!(channel_value(0.25, &t, 0) < 0.25);
    }

    #[test]
    fn values_are_clamped_into_range() {
        let t = Tone { gamma: 9.0, brightness: 99.0, contrast: 1.0, red: 999.0, green: -5.0, blue: f64::NAN }.clamped();
        assert_eq!(t.gamma, GAMMA_MAX);
        assert_eq!(t.brightness, BRIGHTNESS_MAX);
        assert_eq!(t.contrast, CONTRAST_MIN);
        assert_eq!(t.red, CHANNEL_MAX);
        assert_eq!(t.green, CHANNEL_MIN);
        assert_eq!(t.blue, 100.0);
    }

    #[test]
    fn blend_halfway() {
        let a = identity_ramp();
        let b = build_ramp(&Tone { gamma: 3.0, ..Tone::default() });
        let m = blend_ramp(&a, &b, 0.5);
        let expected = ((a[2][40] as f64 + b[2][40] as f64) / 2.0).round() as u16;
        assert_eq!(m[2][40], expected);
        assert_eq!(blend_ramp(&a, &b, 0.0), a);
        assert_eq!(blend_ramp(&a, &b, 1.0), b);
    }

    #[test]
    fn deviation_of_identity_is_zero() {
        assert_eq!(max_deviation(&identity_ramp()), 0);
        assert!(max_deviation(&build_ramp(&Tone { gamma: 3.0, ..Tone::default() })) > 10000);
    }
}
