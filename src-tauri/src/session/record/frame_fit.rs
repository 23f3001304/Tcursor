//! The pure geometry of the OBS-style fit: where a capture frame of one size lands inside the
//! encoder's fixed output canvas. Deliberately free of D3D11 and of every Windows type, so it
//! is the part of `frame_scaler.rs` that can actually be tested - the video processor that
//! performs the fit needs a live GPU capture and cannot be.

/// The destination rect `(left, top, right, bottom)` for a `src`-sized frame scaled into a
/// `dst`-sized canvas: aspect preserved and centred, so whatever is left over becomes the
/// black bars (top/bottom for a relatively wider source, left/right for a taller one).
///
/// Everything is integer: comparing `sw/sh` against `dw/dh` in floats would round a
/// one-pixel-off aspect into a distortion, or a distortion into an exact fit.
pub fn letterbox(src: (u32, u32), dst: (u32, u32)) -> (i32, i32, i32, i32) {
    let (sw, sh) = (u64::from(src.0.max(1)), u64::from(src.1.max(1)));
    let (dw, dh) = (u64::from(dst.0.max(1)), u64::from(dst.1.max(1)));
    let (w, h) = if sw * dh >= dw * sh {
        (dw, (sh * dw / sw).clamp(1, dh)) // at least as wide as the canvas: full width
    } else {
        ((sw * dh / sh).clamp(1, dw), dh) // taller: full height
    };
    let (x, y) = ((dw - w) / 2, (dh - h) / 2);
    (x as i32, y as i32, (x + w) as i32, (y + h) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The common case by far: the capture never resized, so the fit is the identity and the
    /// picture is not resampled at all.
    #[test]
    fn equal_sizes_fill_the_canvas_exactly() {
        assert_eq!(letterbox((1920, 1080), (1920, 1080)), (0, 0, 1920, 1080));
        assert_eq!(letterbox((1, 1), (1, 1)), (0, 0, 1, 1));
    }

    /// A window that grew (or a display that changed to a bigger mode) scales DOWN into the
    /// canvas the encoder was built for - the whole picture stays visible.
    #[test]
    fn a_grown_frame_scales_down_to_fit() {
        assert_eq!(letterbox((2560, 1440), (1920, 1080)), (0, 0, 1920, 1080));
        assert_eq!(letterbox((3840, 2160), (1280, 720)), (0, 0, 1280, 720));
    }

    /// A window that shrank scales UP rather than being padded into a corner, so the recording
    /// keeps filling the frame instead of suddenly showing a small image on black.
    #[test]
    fn a_shrunk_frame_scales_up_to_fit() {
        assert_eq!(letterbox((960, 540), (1920, 1080)), (0, 0, 1920, 1080));
    }

    /// Relatively wider than the canvas: full width, equal bars above and below.
    #[test]
    fn a_wider_frame_gets_bars_top_and_bottom() {
        assert_eq!(letterbox((1920, 1000), (1920, 1080)), (0, 40, 1920, 1040));
        assert_eq!(letterbox((1000, 250), (1000, 500)), (0, 125, 1000, 375));
    }

    /// Relatively taller than the canvas: full height, equal bars left and right.
    #[test]
    fn a_taller_frame_gets_bars_left_and_right() {
        assert_eq!(letterbox((1920, 1200), (1920, 1080)), (96, 0, 1824, 1080));
        assert_eq!(letterbox((250, 1000), (500, 1000)), (125, 0, 375, 1000));
    }

    /// The bug this exists for: switching Chrome tabs toggles the bookmarks bar, so the capture
    /// loses ~30 rows mid-take. The fit keeps the full width and a thin black band, instead of
    /// the picture freezing on the last full-height frame for the rest of the recording.
    #[test]
    fn the_bookmarks_bar_case_keeps_the_full_width() {
        assert_eq!(letterbox((1600, 870), (1600, 900)), (0, 15, 1600, 885));
    }

    /// The invariants that make this a fit and not a stretch, over a spread of shapes: the rect
    /// stays inside the canvas, touches one pair of its edges, is centred (opposite bars within
    /// a pixel, since an odd leftover cannot split evenly), and matches the ideal uniformly
    /// scaled rect to within a pixel - i.e. one scale factor for both axes, never two.
    #[test]
    fn the_fit_is_centred_and_never_distorts() {
        let dst = (1920u32, 1080u32);
        for src in [(640, 480), (1920, 1080), (3440, 1440), (600, 2000), (1280, 800), (17, 991)] {
            let (l, t, r, b) = letterbox(src, dst);
            let (w, h) = (f64::from(r - l), f64::from(b - t));
            assert!(l >= 0 && t >= 0 && r <= dst.0 as i32 && b <= dst.1 as i32, "{src:?} escapes");
            assert!((l - (dst.0 as i32 - r)).abs() <= 1, "{src:?} not centred horizontally");
            assert!((t - (dst.1 as i32 - b)).abs() <= 1, "{src:?} not centred vertically");
            assert!(w as u32 == dst.0 || h as u32 == dst.1, "{src:?} does not touch either edge");
            let scale = (f64::from(dst.0) / f64::from(src.0)).min(f64::from(dst.1) / f64::from(src.1));
            assert!((w - f64::from(src.0) * scale).abs() <= 1.0, "{src:?} width off: {w}");
            assert!((h - f64::from(src.1) * scale).abs() <= 1.0, "{src:?} height off: {h}");
        }
    }
}
