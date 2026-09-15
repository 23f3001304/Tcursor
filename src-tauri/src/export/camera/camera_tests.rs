use super::*;
use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

const STEP: f32 = 16.0;

fn region() -> ZoomRegion {
    ZoomRegion {
        start_ms: 0,
        end_ms: 2000,
        zoom_in_ms: 300,
        zoom_out_ms: 300,
        target_scale: 2.0,
        anchor: FramePoint { x: 400, y: 300 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    }
}

#[test]
fn no_region_is_full_frame() {
    let mut s = CameraSim::new(800, 600);
    let c = s.step(
        0,
        STEP,
        FramePoint { x: 400, y: 300 },
        &[],
        &ZoomConfig::default(),
    );
    assert_eq!(c.scale, 1.0);
    assert!((c.cx - 400.0).abs() < 1.0 && (c.cy - 300.0).abs() < 1.0);
}

#[test]
fn scale_is_one_at_region_end_and_after() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![region()];
    for t in (0..=2000).step_by(16) {
        let _ = s.step(t, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg);
    }
    assert!(
        (s.step(2000, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg)
            .scale
            - 1.0)
            .abs()
            < 1e-3,
        "scale must be exactly 1 at end_ms"
    );
    assert!(
        (s.step(2100, STEP, FramePoint { x: 400, y: 300 }, &[], &cfg)
            .scale
            - 1.0)
            .abs()
            < 1e-3,
        "scale stays 1 after the region"
    );
}

#[test]
fn scale_reaches_target_during_hold() {
    let mut s = CameraSim::new(800, 600);
    let c = s.step(
        1000,
        STEP,
        FramePoint { x: 400, y: 300 },
        &[region()],
        &ZoomConfig::default(),
    );
    assert!((c.scale - 2.0).abs() < 1e-3, "hold scale equals target");
}

#[test]
fn ramp_in_is_monotonic_and_bounded() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![region()];
    let a = s
        .step(0, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg)
        .scale;
    let b = s
        .step(150, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg)
        .scale;
    assert!(
        a < b && b < 2.0 && a >= 1.0,
        "in-ramp climbs from 1 toward target: a={a} b={b}"
    );
}

#[test]
fn durations_that_exceed_span_are_scaled_to_fit() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![ZoomRegion {
        start_ms: 0,
        end_ms: 400,
        ..region()
    }];
    for t in (0..=400).step_by(16) {
        let _ = s.step(t, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg);
    }
    assert!(
        (s.step(400, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg)
            .scale
            - 1.0)
            .abs()
            < 1e-3
    );
}

#[test]
fn newer_region_preempts_older_overlap() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let a = ZoomRegion {
        start_ms: 0,
        end_ms: 2000,
        anchor: FramePoint { x: 100, y: 100 },
        ..region()
    };
    let b = ZoomRegion {
        start_ms: 1000,
        end_ms: 3000,
        anchor: FramePoint { x: 700, y: 500 },
        ..region()
    };
    let r = vec![a, b];
    let mut c = Camera {
        cx: 0.0,
        cy: 0.0,
        scale: 1.0,
    };
    for t in (1000..1300).step_by(16) {
        c = s.step(t, STEP, FramePoint { x: 700, y: 500 }, &r, &cfg);
    }
    assert!(
        c.cx > 400.0,
        "expected to move toward b.anchor.x=700, got {}",
        c.cx
    );
}

#[test]
fn center_clamps_inside_frame() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let r = vec![ZoomRegion {
        anchor: FramePoint { x: 0, y: 0 },
        ..region()
    }];
    let mut c = Camera {
        cx: 0.0,
        cy: 0.0,
        scale: 1.0,
    };
    for t in (0..1000).step_by(16) {
        c = s.step(t, STEP, FramePoint { x: 0, y: 0 }, &r, &cfg);
    }
    assert!(c.cx >= 200.0 - 1.0 && c.cy >= 150.0 - 1.0);
}

#[test]
fn highest_layer_wins_not_most_recent() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let a = ZoomRegion {
        layer: 1,
        anchor: FramePoint { x: 100, y: 100 },
        ..region()
    };
    let b = ZoomRegion {
        layer: 0,
        start_ms: 500,
        end_ms: 2500,
        anchor: FramePoint { x: 700, y: 500 },
        ..region()
    };
    let r = vec![a, b];
    let mut c = Camera {
        cx: 0.0,
        cy: 0.0,
        scale: 1.0,
    };
    for t in (500..800).step_by(16) {
        c = s.step(t, STEP, FramePoint { x: 100, y: 100 }, &r, &cfg);
    }
    assert!(
        c.cx < 400.0,
        "expected layer-1 region (a) to win over layer-0 region (b), got cx={}",
        c.cx
    );
}
#[test]
fn the_camera_never_zooms_out_past_full_frame() {
    let mut s = CameraSim::new(800, 600);
    let (cfg, r) = (
        ZoomConfig::default(),
        vec![ZoomRegion {
            easing: Easing::Linear,
            easing_out: Easing::Linear,
            ..region()
        }],
    );
    let mut low = f32::MAX;
    for t in (0..=2600).step_by(16) {
        low = low.min(
            s.step(t, STEP, FramePoint { x: 400, y: 300 }, &r, &cfg)
                .scale,
        );
    }
    assert!(
        low >= 1.0,
        "the camera zoomed out past the frame: scale {low}"
    );
}

#[path = "camera_handoff_tests.rs"]
mod camera_handoff_tests;
