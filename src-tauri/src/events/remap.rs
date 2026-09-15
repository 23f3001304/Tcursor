use crate::session::record::frame_fit::letterbox;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Remap {
    from_origin: (i32, i32),
    from_size: (u32, u32),
    canvas_origin: (i32, i32),
    canvas_size: (u32, u32),
    fit: (i32, i32, i32, i32),
}

impl Remap {
    pub fn for_switch(
        new_origin: (i32, i32),
        new_size: (u32, u32),
        canvas_origin: (i32, i32),
        canvas_size: (u32, u32),
    ) -> Self {
        Self {
            from_origin: new_origin,
            from_size: (new_size.0.max(1), new_size.1.max(1)),
            canvas_origin,
            canvas_size: (canvas_size.0.max(1), canvas_size.1.max(1)),
            fit: letterbox(new_size, canvas_size),
        }
    }

    pub fn apply(&self, x: i32, y: i32) -> (i32, i32) {
        let (l, t, r, b) = self.fit;
        (
            axis(
                x,
                self.from_origin.0,
                self.from_size.0,
                (l, r - l),
                self.canvas_origin.0,
                self.canvas_size.0,
            ),
            axis(
                y,
                self.from_origin.1,
                self.from_size.1,
                (t, b - t),
                self.canvas_origin.1,
                self.canvas_size.1,
            ),
        )
    }
}

fn axis(
    v: i32,
    origin: i32,
    size: u32,
    fit: (i32, i32),
    canvas_origin: i32,
    canvas_len: u32,
) -> i32 {
    let (fit_start, fit_len) = fit;
    let local = i64::from(v) - i64::from(origin);
    let mapped = local * i64::from(fit_len) / i64::from(size.max(1))
        + i64::from(fit_start)
        + i64::from(canvas_origin);
    let lo = i64::from(canvas_origin);
    let hi = lo + i64::from(canvas_len.max(1)) - 1;
    mapped.clamp(lo, hi) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_same_size_display_is_the_identity_plus_the_origin_shift() {
        let r = Remap::for_switch((1920, 0), (1920, 1080), (0, 0), (1920, 1080));
        assert_eq!(r.apply(1920, 0), (0, 0));
        assert_eq!(r.apply(1920 + 300, 200), (300, 200));
        assert_eq!(r.apply(1920 + 1919, 1079), (1919, 1079));
    }

    #[test]
    fn the_canvas_origin_is_carried_into_the_result() {
        let r = Remap::for_switch((0, 0), (1920, 1080), (-1920, 0), (1920, 1080));
        assert_eq!(r.apply(300, 200), (-1620, 200));
    }

    #[test]
    fn a_smaller_sixteen_ten_display_is_scaled_and_barred() {
        let r = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));
        assert_eq!(letterbox((1280, 800), (1920, 1080)), (96, 0, 1824, 1080));
        assert_eq!(r.apply(1920, 0), (96, 0));
        assert_eq!(r.apply(1920 + 640, 400), (960, 540));
        assert_eq!(r.apply(1920 + 1279, 799), (1822, 1078));
    }

    #[test]
    fn a_point_off_the_new_display_is_clamped_to_the_canvas() {
        let r = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));
        assert_eq!(r.apply(1920 - 1000, 400), (0, 540));
        assert_eq!(r.apply(1920 + 5000, 5000), (1919, 1079));
        assert_eq!(r.apply(0, -10_000), (0, 0));
    }

    #[test]
    fn the_clamp_follows_the_canvas_origin() {
        let r = Remap::for_switch((0, 0), (1920, 1080), (-1920, -100), (1920, 1080));
        assert_eq!(r.apply(-9_000, -9_000), (-1920, -100));
        assert_eq!(r.apply(9_000, 9_000), (-1, 979));
    }

    #[test]
    fn a_bigger_display_scales_down_with_no_bars() {
        let r = Remap::for_switch((0, 0), (3840, 2160), (0, 0), (1920, 1080));
        assert_eq!(r.apply(0, 0), (0, 0));
        assert_eq!(r.apply(1920, 1080), (960, 540));
        assert_eq!(r.apply(3839, 2159), (1919, 1079));
    }
}
