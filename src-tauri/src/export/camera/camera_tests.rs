// Tests for export::camera, split into their own file so camera.rs stays under the size limit.
use super::*;
use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

fn region() -> ZoomRegion {
    ZoomRegion { start_ms: 0, end_ms: 2000, zoom_in_ms: 300, zoom_out_ms: 300,
        target_scale: 2.0, anchor: FramePoint { x: 400, y: 300 }, easing: Easing::Smooth, layer: 0 }
}

#[test]
fn no_region_is_full_frame() {
    let mut s = CameraSim::new(800, 600);
    let c = s.step(0, FramePoint { x: 400, y: 300 }, &[], &ZoomConfig::default());
    assert_eq!(c.scale, 1.0);
    assert!((c.cx - 400.0).abs() < 1.0 && (c.cy - 300.0).abs() < 1.0);
}

#[test]
fn scale_is_one_at_region_end_and_after() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![region()];
    // walk up to the end so the (stateful) center advances naturally
    let mut sc = 0.0;
    for t in (0..=2000).step_by(16) { sc = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg).scale; }
    assert!((s.step(2000, FramePoint { x: 400, y: 300 }, &r, &cfg).scale - 1.0).abs() < 1e-3, "scale must be exactly 1 at end_ms");
    assert!((s.step(2100, FramePoint { x: 400, y: 300 }, &[], &cfg).scale - 1.0).abs() < 1e-3, "scale stays 1 after the region");
    let _ = sc;
}

#[test]
fn scale_reaches_target_during_hold() {
    let mut s = CameraSim::new(800, 600);
    let c = s.step(1000, FramePoint { x: 400, y: 300 }, &[region()], &ZoomConfig::default());
    assert!((c.scale - 2.0).abs() < 1e-3, "hold scale equals target");
}

#[test]
fn ramp_in_is_monotonic_and_bounded() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![region()];
    let a = s.step(0, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
    let b = s.step(150, FramePoint { x: 400, y: 300 }, &r, &cfg).scale;
    assert!(a < b && b < 2.0 && a >= 1.0, "in-ramp climbs from 1 toward target: a={a} b={b}");
}

#[test]
fn durations_that_exceed_span_are_scaled_to_fit() {
    // in+out = 600 > span 400: must still reach 1.0 exactly at end and never NaN.
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![ZoomRegion { start_ms: 0, end_ms: 400, ..region() }];
    for t in (0..=400).step_by(16) { let _ = s.step(t, FramePoint { x: 400, y: 300 }, &r, &cfg); }
    assert!((s.step(400, FramePoint { x: 400, y: 300 }, &r, &cfg).scale - 1.0).abs() < 1e-3);
}

#[test]
fn newer_region_preempts_older_overlap() {
    // Two overlapping regions: at t in both, the later-starting one wins.
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let a = ZoomRegion { start_ms: 0, end_ms: 2000, anchor: FramePoint { x: 100, y: 100 }, ..region() };
    let b = ZoomRegion { start_ms: 1000, end_ms: 3000, anchor: FramePoint { x: 700, y: 500 }, ..region() };
    let r = vec![a, b];
    // Drive well into b's zoom-in window; the center should track b's anchor side.
    let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (1000..1300).step_by(16) { c = s.step(t, FramePoint { x: 700, y: 500 }, &r, &cfg); }
    assert!(c.cx > 400.0, "expected to move toward b.anchor.x=700, got {}", c.cx);
}

#[test]
fn center_clamps_inside_frame() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![ZoomRegion { anchor: FramePoint { x: 0, y: 0 }, ..region() }];
    let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (0..1000).step_by(16) { c = s.step(t, FramePoint { x: 0, y: 0 }, &r, &cfg); }
    // at scale ~2 the half-view is 200x150; center must stay >= that
    assert!(c.cx >= 200.0 - 1.0 && c.cy >= 150.0 - 1.0);
}

#[test]
fn highest_layer_wins_not_most_recent() {
    // b is added AFTER a (higher index) but on a LOWER layer - a must win, since layer
    // is the priority now, not array position.
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let a = ZoomRegion { layer: 1, anchor: FramePoint { x: 100, y: 100 }, ..region() };
    let b = ZoomRegion { layer: 0, start_ms: 500, end_ms: 2500, anchor: FramePoint { x: 700, y: 500 }, ..region() };
    let r = vec![a, b];
    let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (500..800).step_by(16) { c = s.step(t, FramePoint { x: 100, y: 100 }, &r, &cfg); }
    // Should track toward a's anchor (100,100), not b's (700,500), despite b being active too.
    assert!(c.cx < 400.0, "expected layer-1 region (a) to win over layer-0 region (b), got cx={}", c.cx);
}

#[test]
fn handoff_eases_from_current_camera_state_not_frame_center() {
    // Region A (layer 0): active 0..4000, target scale 1.5, holding the whole time.
    // Region B (layer 1, higher priority): active 2000..3000, target scale 2.5.
    // At t=2000 B takes over. The reported bug: the old code always eased the zoom-in
    // ramp from frame-center/scale-1, so the camera would snap toward center before
    // ramping to B - instead it must ease from wherever A left it.
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let regions = vec![
        ZoomRegion { start_ms: 0, end_ms: 4000, zoom_in_ms: 300, zoom_out_ms: 300,
            target_scale: 1.5, anchor: FramePoint { x: 700, y: 500 }, easing: Easing::Smooth, layer: 0 },
        ZoomRegion { start_ms: 2000, end_ms: 3000, zoom_in_ms: 200, zoom_out_ms: 200,
            target_scale: 2.5, anchor: FramePoint { x: 100, y: 100 }, easing: Easing::Smooth, layer: 1 },
    ];
    let mut before = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (0..2000).step_by(16) {
        before = s.step(t, FramePoint { x: 700, y: 500 }, &regions, &cfg);
    }
    assert!((before.scale - 1.5).abs() < 0.01, "region A should be fully held at its target scale: {}", before.scale);

    let after = s.step(2016, FramePoint { x: 700, y: 500 }, &regions, &cfg);
    assert!((after.scale - before.scale).abs() < 0.2,
        "camera snapped on handoff instead of easing smoothly: before={} after={}", before.scale, after.scale);
}

#[test]
fn handoff_reaches_the_new_winners_target_once_its_transition_completes() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let regions = vec![
        ZoomRegion { start_ms: 0, end_ms: 4000, zoom_in_ms: 300, zoom_out_ms: 300,
            target_scale: 1.5, anchor: FramePoint { x: 700, y: 500 }, easing: Easing::Smooth, layer: 0 },
        ZoomRegion { start_ms: 2000, end_ms: 3000, zoom_in_ms: 200, zoom_out_ms: 200,
            target_scale: 2.5, anchor: FramePoint { x: 100, y: 100 }, easing: Easing::Smooth, layer: 1 },
    ];
    let mut c = Camera { cx: 0.0, cy: 0.0, scale: 1.0 };
    for t in (0..2400).step_by(16) { c = s.step(t, FramePoint { x: 700, y: 500 }, &regions, &cfg); }
    // 400ms after the t=2000 handoff, well past B's 200ms zoom_in_ms transition window.
    assert!((c.scale - 2.5).abs() < 0.05, "expected to reach B's target scale 2.5: {}", c.scale);
}
