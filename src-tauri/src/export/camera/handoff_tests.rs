// Tests for the handoff blend: the two end conditions the Hermite velocity term must not break
// (start at the captured pose, land exactly on the target) and the defect it exists to remove -
// a moving camera being stopped dead at the seam (H2).
use super::*;
use crate::export::camera::CameraSim;
use crate::export::types::{FramePoint, ZoomConfig, ZoomRegion};

/// The exact 60fps frame period every caller hands `CameraSim::step` (`render::OUT_STEP_MS`).
const STEP: f32 = crate::export::camera::follow::REF_STEP_MS;

fn pose(cx: f32) -> Camera { Camera { cx, cy: 500.0, scale: 2.0 } }

#[test]
fn the_blend_carries_the_captured_velocity_into_its_first_step() {
    // Position-only easing gives `ease'(0) = 0` for smoothstep: the first frame moved the camera
    // by ~0.06px instead of the 10px it was already travelling. The velocity term restores it.
    // `open` backdates the seam by one step, so the FIRST `blend` is already one step in - a
    // blend that re-emitted `from` here would be the very stall this is about.
    let v = 0.6f32; // px/ms
    // Target where the camera already is, so the velocity term is the ONLY motion: over a 1ms
    // step it must be free flight, to within the Hermite basis' own curvature.
    let mut tr = Transition::open(pose(100.0), (v, 0.0, 0.0), 1.0, 400.0, Easing::Smooth);
    let d1 = tr.blend(1.0, pose(100.0)).unwrap().cx - 100.0;
    assert!((d1 - v).abs() < 0.01 * v, "1ms of free flight should be {v}px, got {d1}");
    // And over a real export frame toward a target 800px away it must be at least that.
    let mut tr = Transition::open(pose(100.0), (v, 0.0, 0.0), STEP, 400.0, Easing::Smooth);
    let d = tr.blend(STEP, pose(900.0)).unwrap().cx - 100.0;
    assert!(d >= v * STEP, "first step stalled: {d}px vs {}px of free flight", v * STEP);
}

#[test]
fn the_blend_still_lands_exactly_on_the_target() {
    // Non-negotiable: whatever the entry velocity, a handoff must arrive at its `dur` - otherwise
    // the incoming region's own ramp and the blend disagree about where the camera ends up. Run
    // it exactly as `CameraSim` does: one `blend` per frame at the exact frame period.
    for v in [-2.0f32, 0.0, 0.6, 3.0] {
        let mut tr = Transition::open(pose(100.0), (v, v, v * 0.01), STEP, 400.0, Easing::Smooth);
        let (mut last, mut n) = (pose(100.0), 0);
        while let Some(c) = tr.blend(STEP, pose(900.0)) { last = c; n += 1; }
        let ran = n as f32 * STEP;
        assert!((400.0..=400.0 + 2.0 * STEP).contains(&ran),
            "v={v}: the blend ran {n} frames ({ran:.1}ms), want ~400ms + the backdated step");
        assert!((last.cx - 900.0).abs() < 6.0, "v={v}: the last blended frame must be ~the target, got {}", last.cx);
        assert!(tr.blend(STEP, pose(900.0)).is_none(), "v={v}: a finished blend must stay finished");
    }
}

#[test]
fn a_moving_camera_is_not_stopped_dead_by_a_handoff() {
    // A is a follow region panning steadily right; higher-layer B takes over mid-pan at t=2000.
    let cfg = ZoomConfig::default();
    let cur = |t: u32| FramePoint { x: 300 + (t as f32 * 0.6) as i32, y: 540 };
    let a = ZoomRegion { start_ms: 0, end_ms: 6000, zoom_in_ms: 350, zoom_out_ms: 450,
        target_scale: 2.2, anchor: FramePoint { x: 960, y: 540 }, easing: Easing::Smooth, layer: 0,
        cam_action: None, follow_cursor: true };
    let b = ZoomRegion { start_ms: 2000, end_ms: 5000, layer: 1, anchor: FramePoint { x: 1500, y: 540 },
        follow_cursor: false, ..a };
    let mut s = CameraSim::new(1920, 1080);
    let (mut ts, mut xs) = (vec![], vec![]);
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > 2300 { break; }
        let c = s.step(t, STEP, cur(t), &[a, b], &cfg);
        ts.push(t); xs.push(c.cx);
    }
    let mean = |lo: u32, hi: u32| {
        let v: Vec<f32> = (1..xs.len()).filter(|i| (lo..=hi).contains(&ts[*i]))
            .map(|i| (xs[i] - xs[i - 1]).abs()).collect();
        v.iter().sum::<f32>() / v.len().max(1) as f32
    };
    let (before, after) = (mean(1850, 1990), mean(2000, 2080));
    assert!(after > before * 0.8, "the handoff still stalls the pan: {before:.2} -> {after:.2} px/frame");
}
