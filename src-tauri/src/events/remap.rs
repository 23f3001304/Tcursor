//! Where a point on the display a take switched TO lands on the display the take STARTED on.
//!
//! A mid-take display switch (`session::record::switch_display`) restarts the capture on another
//! monitor but keeps the encoder, so every later frame is fitted into the first display's canvas
//! by `frame_scaler`/`frame_fit::letterbox` - aspect preserved, centred, black bars. The mouse,
//! click and typing streams, though, carry raw `WH_MOUSE_LL` desktop coordinates, and the export
//! turns those into frame pixels by subtracting the take's ONE `ScreenInfo` origin
//! (`export::coordmap::to_frame`). Left alone they would describe the new monitor's desktop
//! rectangle against the old monitor's picture: the cursor, the click ripples and every zoom
//! anchor would land somewhere else entirely.
//!
//! So the samples are mapped where the pixels are, at capture time (`track::tracker`), and
//! `events.json` keeps one screen. Integer throughout, like `letterbox`, so the mapping of a
//! same-sized display is exactly the identity plus the origin shift and never a rounding drift.
use crate::session::record::frame_fit::letterbox;

/// The fixed mapping installed on the mouse hook for as long as a take is capturing a display
/// other than its own. Copy so the hook can hold it behind its mutex without an allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Remap {
    /// Desktop origin of the display now being captured.
    from_origin: (i32, i32),
    /// Size of the display now being captured (never zero).
    from_size: (u32, u32),
    /// Desktop origin of the take's own display - the one `ScreenInfo` the export subtracts.
    canvas_origin: (i32, i32),
    /// Size of that display, i.e. the encoder canvas (never zero).
    canvas_size: (u32, u32),
    /// `letterbox(from_size, canvas_size)`: where the new display's picture actually sits inside
    /// the canvas, as `(left, top, right, bottom)`.
    fit: (i32, i32, i32, i32),
}

impl Remap {
    /// The mapping for a switch onto a display at `new_origin` sized `new_size`, for a take whose
    /// canvas is the display at `canvas_origin` sized `canvas_size`.
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

    /// Map one desktop point on the captured display to the desktop point on the take's own
    /// display that shows the same pixel. The result stays in ABSOLUTE coordinates because the
    /// export subtracts the take's `ScreenInfo` origin from it afterwards.
    pub fn apply(&self, x: i32, y: i32) -> (i32, i32) {
        let (l, t, r, b) = self.fit;
        (
            axis(x, self.from_origin.0, self.from_size.0, (l, r - l), self.canvas_origin.0, self.canvas_size.0),
            axis(y, self.from_origin.1, self.from_size.1, (t, b - t), self.canvas_origin.1, self.canvas_size.1),
        )
    }
}

/// One axis of `Remap::apply`: local offset on the captured display, scaled into the fit rect,
/// shifted by the rect's offset and the canvas' own origin, then clamped to the canvas. `i64`
/// throughout because `offset * fit_len` overflows `i32` at ordinary desktop sizes.
///
/// The clamp is what keeps a point on a THIRD monitor (or off the desktop entirely) inside the
/// frame the editor knows about, rather than producing an anchor the camera would chase off
/// screen. Integer division truncates toward zero, which is the same half-pixel slack
/// `letterbox` itself carries.
fn axis(v: i32, origin: i32, size: u32, fit: (i32, i32), canvas_origin: i32, canvas_len: u32) -> i32 {
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

    /// The second monitor is the same size as the take's: the mapping is the identity apart from
    /// moving the point off the second monitor's desktop rectangle and onto the first's, which is
    /// exactly what `to_frame`'s origin subtraction then undoes.
    #[test]
    fn a_same_size_display_is_the_identity_plus_the_origin_shift() {
        let r = Remap::for_switch((1920, 0), (1920, 1080), (0, 0), (1920, 1080));
        assert_eq!(r.apply(1920, 0), (0, 0));
        assert_eq!(r.apply(1920 + 300, 200), (300, 200));
        assert_eq!(r.apply(1920 + 1919, 1079), (1919, 1079));
    }

    /// The take's own display sits left of the origin (a monitor arranged to the left of the
    /// primary one has a negative origin): the mapped point keeps that origin, because the
    /// export subtracts it and not zero.
    #[test]
    fn the_canvas_origin_is_carried_into_the_result() {
        let r = Remap::for_switch((0, 0), (1920, 1080), (-1920, 0), (1920, 1080));
        assert_eq!(r.apply(300, 200), (-1620, 200));
    }

    /// A 1280x800 laptop panel (16:10) recorded into a 1920x1080 canvas: `letterbox` gives a
    /// 1728x1080 picture with 96px bars left and right, so the panel's centre is the canvas'
    /// centre, its left edge is the inner edge of the left bar, and nothing is distorted.
    #[test]
    fn a_smaller_sixteen_ten_display_is_scaled_and_barred() {
        let r = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));
        assert_eq!(letterbox((1280, 800), (1920, 1080)), (96, 0, 1824, 1080));
        assert_eq!(r.apply(1920, 0), (96, 0));
        assert_eq!(r.apply(1920 + 640, 400), (960, 540));
        assert_eq!(r.apply(1920 + 1279, 799), (1822, 1078));
    }

    /// A point that is not on the captured display at all - the pointer crossed onto a third
    /// monitor, or the hook fired while it was over the taskbar of another one - is pinned to
    /// the canvas edge instead of being written as an off-frame anchor.
    #[test]
    fn a_point_off_the_new_display_is_clamped_to_the_canvas() {
        let r = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));
        assert_eq!(r.apply(1920 - 1000, 400), (0, 540));
        assert_eq!(r.apply(1920 + 5000, 5000), (1919, 1079));
        assert_eq!(r.apply(0, -10_000), (0, 0));
    }

    /// The same clamp, on a canvas whose own origin is not zero: the edges are the canvas'
    /// edges, not the desktop's.
    #[test]
    fn the_clamp_follows_the_canvas_origin() {
        let r = Remap::for_switch((0, 0), (1920, 1080), (-1920, -100), (1920, 1080));
        assert_eq!(r.apply(-9_000, -9_000), (-1920, -100));
        assert_eq!(r.apply(9_000, 9_000), (-1, 979));
    }

    /// A bigger display scales DOWN into the canvas, so the whole of it stays visible and the
    /// mapped point tracks the picture rather than running off the right edge.
    #[test]
    fn a_bigger_display_scales_down_with_no_bars() {
        let r = Remap::for_switch((0, 0), (3840, 2160), (0, 0), (1920, 1080));
        assert_eq!(r.apply(0, 0), (0, 0));
        assert_eq!(r.apply(1920, 1080), (960, 540));
        assert_eq!(r.apply(3839, 2159), (1919, 1079));
    }
}
