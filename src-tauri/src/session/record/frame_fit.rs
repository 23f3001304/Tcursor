pub fn letterbox(src: (u32, u32), dst: (u32, u32)) -> (i32, i32, i32, i32) {
    let (sw, sh) = (u64::from(src.0.max(1)), u64::from(src.1.max(1)));
    let (dw, dh) = (u64::from(dst.0.max(1)), u64::from(dst.1.max(1)));
    let (w, h) = if sw * dh >= dw * sh {
        (dw, (sh * dw / sw).clamp(1, dh))
    } else {
        ((sw * dh / sh).clamp(1, dw), dh)
    };
    let (x, y) = ((dw - w) / 2, (dh - h) / 2);
    (x as i32, y as i32, (x + w) as i32, (y + h) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_sizes_fill_the_canvas_exactly() {
        assert_eq!(letterbox((1920, 1080), (1920, 1080)), (0, 0, 1920, 1080));
        assert_eq!(letterbox((1, 1), (1, 1)), (0, 0, 1, 1));
    }

    #[test]
    fn a_grown_frame_scales_down_to_fit() {
        assert_eq!(letterbox((2560, 1440), (1920, 1080)), (0, 0, 1920, 1080));
        assert_eq!(letterbox((3840, 2160), (1280, 720)), (0, 0, 1280, 720));
    }

    #[test]
    fn a_shrunk_frame_scales_up_to_fit() {
        assert_eq!(letterbox((960, 540), (1920, 1080)), (0, 0, 1920, 1080));
    }

    #[test]
    fn a_wider_frame_gets_bars_top_and_bottom() {
        assert_eq!(letterbox((1920, 1000), (1920, 1080)), (0, 40, 1920, 1040));
        assert_eq!(letterbox((1000, 250), (1000, 500)), (0, 125, 1000, 375));
    }

    #[test]
    fn a_taller_frame_gets_bars_left_and_right() {
        assert_eq!(letterbox((1920, 1200), (1920, 1080)), (96, 0, 1824, 1080));
        assert_eq!(letterbox((250, 1000), (500, 1000)), (125, 0, 375, 1000));
    }

    #[test]
    fn the_bookmarks_bar_case_keeps_the_full_width() {
        assert_eq!(letterbox((1600, 870), (1600, 900)), (0, 15, 1600, 885));
    }

    #[test]
    fn the_fit_is_centred_and_never_distorts() {
        let dst = (1920u32, 1080u32);
        for src in [
            (640, 480),
            (1920, 1080),
            (3440, 1440),
            (600, 2000),
            (1280, 800),
            (17, 991),
        ] {
            let (l, t, r, b) = letterbox(src, dst);
            let (w, h) = (f64::from(r - l), f64::from(b - t));
            assert!(
                l >= 0 && t >= 0 && r <= dst.0 as i32 && b <= dst.1 as i32,
                "{src:?} escapes"
            );
            assert!(
                (l - (dst.0 as i32 - r)).abs() <= 1,
                "{src:?} not centred horizontally"
            );
            assert!(
                (t - (dst.1 as i32 - b)).abs() <= 1,
                "{src:?} not centred vertically"
            );
            assert!(
                w as u32 == dst.0 || h as u32 == dst.1,
                "{src:?} does not touch either edge"
            );
            let scale =
                (f64::from(dst.0) / f64::from(src.0)).min(f64::from(dst.1) / f64::from(src.1));
            assert!(
                (w - f64::from(src.0) * scale).abs() <= 1.0,
                "{src:?} width off: {w}"
            );
            assert!(
                (h - f64::from(src.1) * scale).abs() <= 1.0,
                "{src:?} height off: {h}"
            );
        }
    }
}
