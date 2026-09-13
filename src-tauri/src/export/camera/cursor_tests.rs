// Tests for the LIVE-cursor zoom-in aim (`ZoomRegion::follow_cursor`), split from
// camera_tests.rs so both stay under the size limit.
use super::*;
use crate::export::types::{Camera, Easing, FramePoint, ZoomConfig, ZoomRegion};

/// This file drives the sim on a 10ms grid, so that is the step length it declares. `step`'s
/// `dt_ms` is the caller's contract - the real frame period - never a rounded timestamp delta.
const STEP: f32 = 10.0;
const FW: u32 = 1920;
const FH: u32 = 1080;

/// A cursor-target zoom as `fromedit` builds one: the stored anchor is the useless
/// screen-centre default `anchor_for` writes for `ZoomTarget::Cursor`.
fn cursor_zoom(start: u32, end: u32) -> ZoomRegion {
    ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: 350, zoom_out_ms: 450,
        target_scale: 2.0, anchor: FramePoint { x: 960, y: 540 }, easing: Easing::Smooth,
        layer: 0, cam_action: None, follow_cursor: true }
}

/// The tolerance a `follow_cursor` ramp's landing is checked against: 40% of the half-viewport at
/// the target scale (the band an anchored region's aim used to be clamped into, before anchored
/// regions aimed only at their anchor). A follow region aims at the cursor in every phase, so its
/// ramp end and its hold start are the same point and any residue must stay well inside one band.
fn clamp_target(c: f32, p: f32, half: f32, s: f32) -> f32 {
    let m = half / s * 0.4;
    let d = p - c;
    if d > m { p - m } else if d < -m { p + m } else { c }
}

#[test]
fn static_cursor_zoom_matches_the_fixed_anchor_trajectory() {
    // Regression guard: with the cursor parked exactly where a fixed anchor sits, the
    // live-cursor aim must reproduce today's trajectory sample for sample. The fix may
    // only change what happens while the cursor MOVES.
    let cfg = ZoomConfig::default();
    let cur = FramePoint { x: 700, y: 500 };
    let fixed = ZoomRegion { anchor: cur, follow_cursor: false, target_scale: 2.2, ..cursor_zoom(0, 2000) };
    let live = cursor_zoom(0, 2000);
    let live = ZoomRegion { target_scale: 2.2, ..live };
    let (mut sa, mut sb) = (CameraSim::new(FW, FH), CameraSim::new(FW, FH));
    for t in (0..=2000).step_by(16) {
        let (a, b) = (sa.step(t, 16.0, cur, &[fixed], &cfg), sb.step(t, 16.0, cur, &[live], &cfg));
        assert!((a.cx - b.cx).abs() < 1e-3 && (a.cy - b.cy).abs() < 1e-3 && (a.scale - b.scale).abs() < 1e-3,
            "trajectory diverged at t={t}: fixed={a:?} live={b:?}");
    }
}

#[test]
fn moving_cursor_zoom_lands_on_the_live_cursor_with_no_second_stage() {
    // The reported bug: the ramp eases toward the STORED anchor, then the hold phase
    // starts cursor-following from there - a visible two-stage move. The cursor travels
    // (960,540) -> (1400,760) across the 350ms ramp and then parks.
    let cfg = ZoomConfig::default();
    let r = [cursor_zoom(0, 3000)];
    let cur = |t: u32| {
        let f = (t.min(350) as f32) / 350.0;
        FramePoint { x: (960.0 + 440.0 * f) as i32, y: (540.0 + 220.0 * f) as i32 }
    };
    let mut s = CameraSim::new(FW, FH);
    let (mut prev, mut samples) = (Camera { cx: 960.0, cy: 540.0, scale: 1.0 }, Vec::new());
    for t in (0..350).step_by(10) { prev = s.step(t, STEP, cur(t), &r, &cfg); samples.push(prev); }
    // (1) Monotonic approach: no reversal and no overshoot past where it lands.
    for w in samples.windows(2) {
        assert!(w[1].cx >= w[0].cx - 1e-3 && w[1].cy >= w[0].cy - 1e-3,
            "ramp reversed direction: {:?} -> {:?}", w[0], w[1]);
    }
    let land = samples[samples.len() - 1];
    assert!(samples.iter().all(|c| c.cx <= land.cx + 1e-3 && c.cy <= land.cy + 1e-3), "ramp overshot its landing");
    // (2) At zin_end the ramp's endpoint IS the hold phase's first target - zero discontinuity.
    let hold = s.step(350, STEP, cur(350), &r, &cfg);
    let (tx, ty) = (clamp_target(prev.cx, 1400.0, 960.0, 2.0), clamp_target(prev.cy, 760.0, 540.0, 2.0));
    assert!((hold.cx - tx).abs() < 2.0 && (hold.cy - ty).abs() < 2.0,
        "zoom-in did not land on the follow target: cam=({},{}) want=({tx},{ty})", hold.cx, hold.cy);
    // (3) Past zin_end, with the cursor parked, motion is only follow damping: a monotone,
    // exponentially decaying settle onto the cursor, never a second stage. The camera ends the
    // ramp a few px short of the cursor (the ramp aims at the LIVE cursor, which was still
    // moving), so it closes that gap - the bound is that gap, not the 250px anchor-to-cursor
    // slide the old two-stage bug produced.
    let mut last = hold;
    for t in (360..1000).step_by(10) {
        let c = s.step(t, STEP, cur(t), &r, &cfg);
        let bound = cfg.follow_damping * ((1400.0 - last.cx).abs()).max((760.0 - last.cy).abs()) + 1e-3;
        assert!((c.cx - last.cx).abs() <= bound && (c.cy - last.cy).abs() <= bound,
            "step at t={t} exceeded the damping bound {bound}: {last:?} -> {c:?}");
        assert!(c.cx >= last.cx - 1e-3 && c.cy >= last.cy - 1e-3, "settle reversed at t={t}: {last:?} -> {c:?}");
        last = c;
    }
    assert!((last.cx - 1400.0).abs() < 1.0 && (last.cy - 760.0).abs() < 1.0,
        "the follow did not settle onto the parked cursor: {last:?}");
    assert!((last.cx - hold.cx).abs() < 20.0 && (last.cy - hold.cy).abs() < 20.0,
        "camera kept sliding after the ramp (second stage): {hold:?} -> {last:?}");
}

#[test]
fn retimed_cursor_zoom_ignores_the_stale_anchor() {
    // Same doc, pill dragged +2s. Two runs whose ONLY difference is the stale anchor must
    // be bit-identical, and both must land on the cursor's position at the NEW time.
    let cfg = ZoomConfig::default();
    let cur = |t: u32| if t < 2000 { FramePoint { x: 500, y: 400 } } else { FramePoint { x: 1300, y: 700 } };
    let stale = [cursor_zoom(2000, 3500)];
    let other = [ZoomRegion { anchor: FramePoint { x: 1900, y: 1000 }, ..stale[0] }];
    let (mut sa, mut sb) = (CameraSim::new(FW, FH), CameraSim::new(FW, FH));
    let mut ramp_end = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (0..=3500).step_by(10) {
        let (a, b) = (sa.step(t, STEP, cur(t), &stale, &cfg), sb.step(t, STEP, cur(t), &other, &cfg));
        assert!((a.cx - b.cx).abs() < 1e-6 && (a.cy - b.cy).abs() < 1e-6,
            "stale anchor still moved the camera at t={t}: {a:?} vs {b:?}");
        if t == 2340 { ramp_end = a; }
    }
    assert!((ramp_end.cx - 1300.0).abs() < 2.0 && (ramp_end.cy - 700.0).abs() < 2.0,
        "retimed zoom did not aim at the cursor at its NEW start: {ramp_end:?}");
    assert!((ramp_end.cx - 500.0).abs() > 300.0, "camera still pulled toward the stale anchor: {ramp_end:?}");
}

#[test]
fn handoff_into_a_cursor_zoom_blends_toward_the_live_cursor() {
    // Composes with the Task 9 handoff rules: while the blend is in flight the incoming
    // region's natural target is its STEADY pose, which for a cursor zoom is the live
    // cursor - not the stored anchor. B's ramp is over by t=2200, so the blend has landed; the
    // hold then closes the last percent of the 600px move (A sits exactly on its anchor now), an
    // exponential settle that is within 2px by t=2400.
    let cfg = ZoomConfig::default();
    let a = ZoomRegion { start_ms: 0, end_ms: 4000, target_scale: 1.5,
        anchor: FramePoint { x: 700, y: 500 }, follow_cursor: false, ..cursor_zoom(0, 4000) };
    let b = ZoomRegion { start_ms: 2000, end_ms: 3000, zoom_in_ms: 200, zoom_out_ms: 200,
        target_scale: 2.5, layer: 1, ..cursor_zoom(2000, 3000) };
    let (r, cur) = ([a, b], FramePoint { x: 1300, y: 700 });
    let mut s = CameraSim::new(FW, FH);
    let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (0..=2400).step_by(10) { c = s.step(t, STEP, cur, &r, &cfg); }
    assert!((c.scale - 2.5).abs() < 0.05, "handoff did not reach B's target scale: {c:?}");
    assert!((c.cx - 1300.0).abs() < 2.0 && (c.cy - 700.0).abs() < 2.0,
        "handoff into a cursor zoom settled somewhere other than the live cursor: {c:?}");
}
