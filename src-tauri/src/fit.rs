//! Borderless windowed, the part every system shares: what a window looks like in the list
//! and where it goes on its screen.

use crate::settings::Fit;
use serde::Serialize;

/// A top-level window of another program, as the Borderless page lists it.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: i64,
    pub title: String,
    /// File name of the program, like `game.exe`. Empty when Windows did not say.
    pub exe: String,
    /// Size of the picture inside the frame, in pixels.
    pub width: i32,
    pub height: i32,
    /// Luna took its frame off (and can put it back).
    pub borderless: bool,
    /// It has a title bar or a sizing frame right now.
    pub framed: bool,
}

/// `x`, `y`, `width`, `height` in screen pixels.
pub type Rect = (i32, i32, i32, i32);

/// "16:9" -> (16, 9).
fn ratio(mode: &str) -> Option<(i64, i64)> {
    let (a, b) = mode.split_once(':')?;
    let (a, b) = (a.trim().parse::<i64>().ok()?, b.trim().parse::<i64>().ok()?);
    (a > 0 && b > 0).then_some((a, b))
}

/// Where the window goes on `screen`. `client` is where its picture sits today, in screen pixels.
/// "stay" leaves the picture exactly there; every other size is kept inside the screen and centred.
pub fn target(fit: &Fit, screen: Rect, client: Rect) -> Rect {
    if fit.mode == "stay" {
        return client;
    }
    let (sx, sy, sw, sh) = screen;
    let (sw, sh) = (sw.max(1), sh.max(1));
    let (w, h) = match fit.mode.as_str() {
        "keep" => (client.2, client.3),
        "custom" => (fit.width, fit.height),
        mode => match ratio(mode) {
            // The biggest box of that shape that fits the screen.
            Some((a, b)) => {
                let w = (sh as i64 * a / b) as i32;
                if w <= sw {
                    (w, sh)
                } else {
                    (sw, (sw as i64 * b / a) as i32)
                }
            }
            None => (sw, sh),
        },
    };
    let w = w.clamp(1, sw);
    let h = h.clamp(1, sh);
    (sx + (sw - w) / 2, sy + (sh - h) / 2, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fit(mode: &str) -> Fit {
        Fit { mode: mode.into(), ..Fit::default() }
    }

    const ULTRAWIDE: Rect = (0, 0, 5120, 1440);

    #[test]
    fn fill_takes_the_whole_screen() {
        assert_eq!(target(&fit("fill"), ULTRAWIDE, (0, 0, 1280, 720)), ULTRAWIDE);
        assert_eq!(target(&fit("fill"), (-1920, 200, 1920, 1080), (0, 0, 800, 600)), (-1920, 200, 1920, 1080));
    }

    #[test]
    fn ratios_centre_the_biggest_box() {
        assert_eq!(target(&fit("16:9"), ULTRAWIDE, (0, 0, 0, 0)), (1280, 0, 2560, 1440));
        assert_eq!(target(&fit("43:18"), ULTRAWIDE, (0, 0, 0, 0)), (840, 0, 3440, 1440));
        // A ratio wider than the screen fills the width instead.
        assert_eq!(target(&fit("32:9"), (0, 0, 1920, 1080), (0, 0, 0, 0)), (0, 270, 1920, 540));
    }

    #[test]
    fn keep_and_custom_stay_inside_the_screen() {
        assert_eq!(target(&fit("keep"), ULTRAWIDE, (0, 0, 1920, 1080)), (1600, 180, 1920, 1080));
        let big = Fit { mode: "custom".into(), width: 8000, height: 900, ..Fit::default() };
        assert_eq!(target(&big, ULTRAWIDE, (0, 0, 0, 0)), (0, 270, 5120, 900));
    }

    #[test]
    fn stay_keeps_size_and_place() {
        let picture = (412, 230, 1920, 1080);
        assert_eq!(target(&fit("stay"), ULTRAWIDE, picture), picture);
    }

    #[test]
    fn unknown_mode_fills() {
        assert_eq!(target(&fit("banana"), ULTRAWIDE, (0, 0, 10, 10)), ULTRAWIDE);
        assert_eq!(target(&fit("0:9"), ULTRAWIDE, (0, 0, 10, 10)), ULTRAWIDE);
    }
}
