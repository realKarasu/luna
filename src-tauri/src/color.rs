//! Colour matrices for hue, saturation, vibrance and the "holo" sight filters.
//! Pure code, unit tested. `src/lib/math.ts` mirrors these formulas for the preview.
//!
//! Convention: column vectors, `out = M * [r, g, b, a, 1]`.
//! The Magnification API uses row vectors (`[r g b a 1] * T`), so `to_mag` transposes.

use serde::{Deserialize, Serialize};

pub type Mat5 = [[f64; 5]; 5];

pub const HUE_MIN: f64 = -180.0;
pub const HUE_MAX: f64 = 180.0;
pub const SAT_MIN: f64 = 0.0;
pub const SAT_MAX: f64 = 350.0;
pub const VIB_MIN: f64 = 0.0;
pub const VIB_MAX: f64 = 250.0;

// Luma weights (Rec. 709), the same ones used by SVG/CSS colour filters.
const LR: f64 = 0.2126;
const LG: f64 = 0.7152;
const LB: f64 = 0.0722;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ColorParams {
    /// -180 to +180 degrees.
    pub hue: f64,
    /// 0 to 350 %.
    pub saturation: f64,
    /// 0 to 250 %.
    pub vibrance: f64,
}

impl Default for ColorParams {
    fn default() -> Self {
        ColorParams { hue: 0.0, saturation: 100.0, vibrance: 100.0 }
    }
}

impl ColorParams {
    pub fn clamped(&self) -> ColorParams {
        let f = |v: f64, d: f64| if v.is_finite() { v } else { d };
        ColorParams {
            hue: f(self.hue, 0.0).clamp(HUE_MIN, HUE_MAX),
            saturation: f(self.saturation, 100.0).clamp(SAT_MIN, SAT_MAX),
            vibrance: f(self.vibrance, 100.0).clamp(VIB_MIN, VIB_MAX),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holo {
    Off,
    /// Pushes green sights toward black.
    GreenTrueBlack,
    /// Pushes red sights toward black.
    RedBlackHolo,
}

impl Holo {
    pub fn from_name(s: &str) -> Holo {
        match s {
            "greenTrueBlack" => Holo::GreenTrueBlack,
            "redBlackHolo" => Holo::RedBlackHolo,
            _ => Holo::Off,
        }
    }
}

pub fn identity() -> Mat5 {
    let mut m = [[0.0; 5]; 5];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

/// `a * b`: applying the result equals applying `b` first, then `a`.
pub fn mul(a: &Mat5, b: &Mat5) -> Mat5 {
    let mut out = [[0.0; 5]; 5];
    for i in 0..5 {
        for j in 0..5 {
            out[i][j] = (0..5).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    out
}

/// Saturation matrix, `s` is a factor (1.0 = unchanged, 0.0 = grey).
pub fn saturation(s: f64) -> Mat5 {
    let mut m = identity();
    m[0][0] = LR + (1.0 - LR) * s;
    m[0][1] = LG - LG * s;
    m[0][2] = LB - LB * s;
    m[1][0] = LR - LR * s;
    m[1][1] = LG + (1.0 - LG) * s;
    m[1][2] = LB - LB * s;
    m[2][0] = LR - LR * s;
    m[2][1] = LG - LG * s;
    m[2][2] = LB + (1.0 - LB) * s;
    m
}

/// Hue rotation that keeps luma, `deg` in degrees.
pub fn hue(deg: f64) -> Mat5 {
    let (s, c) = deg.to_radians().sin_cos();
    let mut m = identity();
    m[0][0] = LR + c * (1.0 - LR) - s * LR;
    m[0][1] = LG - c * LG - s * LG;
    m[0][2] = LB - c * LB + s * (1.0 - LB);
    m[1][0] = LR - c * LR + s * 0.143;
    m[1][1] = LG + c * (1.0 - LG) + s * 0.140;
    m[1][2] = LB - c * LB - s * 0.283;
    m[2][0] = LR - c * LR - s * (1.0 - LR);
    m[2][1] = LG - c * LG + s * LG;
    m[2][2] = LB + c * (1.0 - LB) + s * LB;
    m
}

/// Sight filter: takes the selected channel's excess over the other two away,
/// so a pure green (or red) dot becomes black while greys stay unchanged.
/// `strength` 0..1.
pub fn holo(mode: Holo, strength: f64) -> Mat5 {
    let k = strength.clamp(0.0, 1.0);
    let mut m = identity();
    let ch = match mode {
        Holo::Off => return m,
        Holo::GreenTrueBlack => 1,
        Holo::RedBlackHolo => 0,
    };
    for j in 0..3 {
        m[ch][j] = if j == ch { 1.0 - k } else { k / 2.0 };
    }
    m
}

/// The saturation factor the matrix must apply for vibrance, given how much
/// the GPU driver (NVIDIA digital vibrance, AMD saturation) already covers on
/// every screen (see `vibrance::common_cover`).
/// `gpu_covers`: `None` = no GPU vibrance, `Some((low, high))` = the GPU handles
/// vibrance between `low` and `high` percent.
pub fn vibrance_matrix_factor(vibrance: f64, gpu_covers: Option<(f64, f64)>) -> f64 {
    let v = vibrance.clamp(VIB_MIN, VIB_MAX);
    match gpu_covers {
        None => v / 100.0,
        Some((low, high)) => {
            if v > high {
                v / high
            } else if v < low {
                v / low.max(1.0)
            } else {
                1.0
            }
        }
    }
}

/// Full colour matrix. Order: holo filter on the game's colours, then hue, then saturation.
pub fn color_matrix(p: &ColorParams, vib_factor: f64, holo_mode: Holo, holo_strength: f64) -> Mat5 {
    let p = p.clamped();
    let s = (p.saturation / 100.0) * vib_factor.max(0.0);
    let m = mul(&hue(p.hue), &holo(holo_mode, holo_strength));
    mul(&saturation(s), &m)
}

pub fn is_identity(m: &Mat5) -> bool {
    let id = identity();
    (0..5).all(|i| (0..5).all(|j| (m[i][j] - id[i][j]).abs() < 1e-6))
}

/// Flattened, transposed matrix in the layout `MAGCOLOREFFECT` expects.
pub fn to_mag(m: &Mat5) -> [f32; 25] {
    let mut out = [0f32; 25];
    for i in 0..5 {
        for j in 0..5 {
            out[i * 5 + j] = m[j][i] as f32;
        }
    }
    out
}

/// Applies the matrix to an RGB colour (0..1), clamped.
pub fn apply(m: &Mat5, rgb: [f64; 3]) -> [f64; 3] {
    let v = [rgb[0], rgb[1], rgb[2], 1.0, 1.0];
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (0..5).map(|k| m[i][k] * v[k]).sum::<f64>().clamp(0.0, 1.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    #[test]
    fn neutral_params_give_identity() {
        let m = color_matrix(&ColorParams::default(), 1.0, Holo::Off, 0.0);
        assert!(is_identity(&m));
    }

    #[test]
    fn zero_saturation_gives_grey() {
        let m = saturation(0.0);
        let out = apply(&m, [0.9, 0.2, 0.1]);
        assert!((out[0] - out[1]).abs() < 1e-9 && (out[1] - out[2]).abs() < 1e-9);
        let luma = 0.9 * LR + 0.2 * LG + 0.1 * LB;
        assert!((out[0] - luma).abs() < 1e-9);
    }

    #[test]
    fn saturation_keeps_greys() {
        for s in [0.0, 0.5, 2.0, 3.5] {
            assert!(close(apply(&saturation(s), [0.4, 0.4, 0.4]), [0.4, 0.4, 0.4]));
        }
    }

    #[test]
    fn saturation_rows_sum_to_one() {
        let m = saturation(2.7);
        for row in m.iter().take(3) {
            assert!((row[0] + row[1] + row[2] - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn hue_zero_and_full_turn_are_identity() {
        assert!(is_identity(&hue(0.0)));
        let m = hue(360.0);
        assert!(close(apply(&m, [0.8, 0.3, 0.1]), [0.8, 0.3, 0.1]));
    }

    #[test]
    fn hue_keeps_greys() {
        for d in [-180.0, -45.0, 16.0, 90.0, 180.0] {
            assert!(close(apply(&hue(d), [0.5, 0.5, 0.5]), [0.5, 0.5, 0.5]));
        }
    }

    #[test]
    fn hue_moves_red_toward_green() {
        let out = apply(&hue(60.0), [1.0, 0.0, 0.0]);
        assert!(out[1] > 0.0 && out[1] > out[2]);
    }

    #[test]
    fn green_true_black_blacks_out_green() {
        let m = holo(Holo::GreenTrueBlack, 1.0);
        assert!(close(apply(&m, [0.0, 1.0, 0.0]), [0.0, 0.0, 0.0]));
        assert!(close(apply(&m, [0.6, 0.6, 0.6]), [0.6, 0.6, 0.6]));
        // The red and blue outputs are untouched.
        let red = apply(&m, [1.0, 0.0, 0.0]);
        assert!((red[0] - 1.0).abs() < 1e-9 && red[2].abs() < 1e-9);
    }

    #[test]
    fn red_black_holo_blacks_out_red() {
        let m = holo(Holo::RedBlackHolo, 1.0);
        assert!(close(apply(&m, [1.0, 0.0, 0.0]), [0.0, 0.0, 0.0]));
        assert!(close(apply(&m, [0.3, 0.3, 0.3]), [0.3, 0.3, 0.3]));
    }

    #[test]
    fn holo_strength_zero_is_identity() {
        assert!(is_identity(&holo(Holo::GreenTrueBlack, 0.0)));
        assert!(is_identity(&holo(Holo::Off, 1.0)));
    }

    #[test]
    fn mag_layout_is_transposed() {
        let mut m = identity();
        m[0][4] = 0.25; // red offset in column-vector form
        let t = to_mag(&m);
        // Row 4 (the constant 1), column 0 (red output) in row-vector form.
        assert!((t[4 * 5] - 0.25).abs() < 1e-7);
        assert_eq!(t[4], 0.0);
    }

    #[test]
    fn mul_order_applies_right_first() {
        let a = saturation(0.0);
        let b = holo(Holo::RedBlackHolo, 1.0);
        let out = apply(&mul(&a, &b), [1.0, 0.0, 0.0]);
        assert!(close(out, [0.0, 0.0, 0.0]));
    }

    #[test]
    fn vibrance_factor_split() {
        assert!((vibrance_matrix_factor(170.0, None) - 1.7).abs() < 1e-9);
        assert!((vibrance_matrix_factor(170.0, Some((0.0, 200.0))) - 1.0).abs() < 1e-9);
        assert!((vibrance_matrix_factor(250.0, Some((0.0, 200.0))) - 1.25).abs() < 1e-9);
        assert!((vibrance_matrix_factor(50.0, Some((100.0, 200.0))) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn params_clamp() {
        let p = ColorParams { hue: 400.0, saturation: -1.0, vibrance: 900.0 }.clamped();
        assert_eq!(p, ColorParams { hue: 180.0, saturation: 0.0, vibrance: 250.0 });
    }
}
