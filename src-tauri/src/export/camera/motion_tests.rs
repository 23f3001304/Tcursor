use super::*;
use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

const STEP: f32 = 16.0;
const CURSOR: FramePoint = FramePoint { x: 400, y: 300 };

fn region(easing: Easing, easing_out: Easing) -> ZoomRegion {
    ZoomRegion {
        start_ms: 0,
        end_ms: 2000,
        zoom_in_ms: 400,
        zoom_out_ms: 400,
        target_scale: 3.0,
        anchor: CURSOR,
        easing,
        easing_out,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    }
}

fn scale_at(r: &[ZoomRegion], t_ms: u32) -> f32 {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let mut out = 1.0;
    for t in (0..=t_ms).step_by(16) {
        out = s.step(t, STEP, CURSOR, r, &cfg).scale;
    }
    out
}

#[test]
fn the_zoom_out_ramp_follows_easing_out_not_easing() {
    let soft = scale_at(&[region(Easing::Smooth, Easing::Smooth)], 1800);
    let hard = scale_at(&[region(Easing::Smooth, Easing::EaseIn)], 1800);
    assert!(
        (soft - 2.0).abs() < 0.1,
        "smooth out ramp should be half way back: {soft}"
    );
    assert!(
        (hard - 1.5).abs() < 0.1,
        "ease_in out ramp should be a quarter of the way back: {hard}"
    );
}

#[test]
fn the_zoom_in_ramp_still_follows_easing() {
    let a = scale_at(&[region(Easing::EaseIn, Easing::Smooth)], 200);
    let b = scale_at(&[region(Easing::EaseIn, Easing::EaseIn)], 200);
    assert_eq!(a, b);
    assert!(
        (a - 1.5).abs() < 0.05,
        "ease_in in-ramp should be a quarter of the way up: {a}"
    );
}

#[test]
fn whatever_the_out_curve_is_the_pill_stays_an_honest_bound() {
    for e in [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
    ] {
        let r = [region(Easing::Smooth, e)];
        assert!(
            (scale_at(&r, 1600) - 3.0).abs() < 0.02,
            "the hold should still be at target for {e:?}"
        );
        assert!(
            (scale_at(&r, 2000) - 1.0).abs() < 0.02,
            "the ramp should land at full frame for {e:?}"
        );
    }
}
