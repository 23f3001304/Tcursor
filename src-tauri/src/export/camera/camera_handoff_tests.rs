use super::*;

#[test]
fn handoff_eases_from_current_camera_state_not_frame_center() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let regions = vec![
        ZoomRegion {
            start_ms: 0,
            end_ms: 4000,
            zoom_in_ms: 300,
            zoom_out_ms: 300,
            target_scale: 1.5,
            anchor: FramePoint { x: 700, y: 500 },
            easing: Easing::Smooth,
            easing_out: Easing::Smooth,
            layer: 0,
            cam_action: None,
            follow_cursor: false,
        },
        ZoomRegion {
            start_ms: 2000,
            end_ms: 3000,
            zoom_in_ms: 200,
            zoom_out_ms: 200,
            target_scale: 2.5,
            anchor: FramePoint { x: 100, y: 100 },
            easing: Easing::Smooth,
            easing_out: Easing::Smooth,
            layer: 1,
            cam_action: None,
            follow_cursor: false,
        },
    ];
    let mut before = Camera {
        cx: 0.0,
        cy: 0.0,
        scale: 1.0,
    };
    for t in (0..2000).step_by(16) {
        before = s.step(t, STEP, FramePoint { x: 700, y: 500 }, &regions, &cfg);
    }
    assert!(
        (before.scale - 1.5).abs() < 0.01,
        "region A should be fully held at its target scale: {}",
        before.scale
    );

    let after = s.step(2016, STEP, FramePoint { x: 700, y: 500 }, &regions, &cfg);
    assert!(
        (after.scale - before.scale).abs() < 0.2,
        "camera snapped on handoff instead of easing smoothly: before={} after={}",
        before.scale,
        after.scale
    );
}

#[test]
fn identical_layer_handoff_is_invisible() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let a = ZoomRegion {
        start_ms: 0,
        end_ms: 6000,
        zoom_in_ms: 300,
        zoom_out_ms: 300,
        target_scale: 2.2,
        anchor: FramePoint { x: 700, y: 500 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    };
    let r = vec![
        a,
        ZoomRegion {
            start_ms: 2000,
            end_ms: 4000,
            zoom_out_ms: 0,
            layer: 1,
            ..a
        },
    ];
    let cur = FramePoint { x: 700, y: 500 };
    let mut prev = s.step(0, STEP, cur, &r, &cfg);
    for t in (16..2000).step_by(16) {
        prev = s.step(t, STEP, cur, &r, &cfg);
    }
    for t in (2000..4000).step_by(16) {
        let c = s.step(t, STEP, cur, &r, &cfg);
        assert!(
            (c.scale - 2.2).abs() < 0.05,
            "scale pulsed at t={t}: {} (want 2.2)",
            c.scale
        );
        assert!(
            (c.cx - prev.cx).abs() < 5.0,
            "center wobbled at t={t}: {} -> {}",
            prev.cx,
            c.cx
        );
        prev = c;
    }
}

#[test]
fn a_fresh_zoom_after_an_exit_ramps_at_its_own_pace() {
    let cfg = ZoomConfig::default();
    let cur = FramePoint { x: 700, y: 500 };
    let b = ZoomRegion {
        start_ms: 3050,
        end_ms: 6000,
        zoom_in_ms: 350,
        zoom_out_ms: 300,
        target_scale: 2.2,
        anchor: FramePoint { x: 700, y: 500 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    };
    let a = ZoomRegion {
        start_ms: 0,
        end_ms: 3000,
        zoom_in_ms: 300,
        zoom_out_ms: 450,
        target_scale: 1.5,
        ..b
    };
    let first = |r: &[ZoomRegion]| {
        let mut s = CameraSim::new(800, 600);
        (0..)
            .map(|k| k * 16)
            .take_while(|t| *t <= 6000)
            .find(|t| s.step(*t, STEP, cur, r, &cfg).scale >= 2.15)
            .unwrap_or(u32::MAX)
    };
    let (after_exit, alone) = (first(&[a, b]), first(&[b]));
    assert!(after_exit.abs_diff(alone) < 32,
        "stale exit transition slowed the new zoom: reached 2.15 at {after_exit}ms vs {alone}ms alone");
}

#[test]
fn handoff_reaches_the_new_winners_target_once_its_transition_completes() {
    let mut s = CameraSim::new(800, 600);
    let cfg = ZoomConfig::default();
    let regions = vec![
        ZoomRegion {
            start_ms: 0,
            end_ms: 4000,
            zoom_in_ms: 300,
            zoom_out_ms: 300,
            target_scale: 1.5,
            anchor: FramePoint { x: 700, y: 500 },
            easing: Easing::Smooth,
            easing_out: Easing::Smooth,
            layer: 0,
            cam_action: None,
            follow_cursor: false,
        },
        ZoomRegion {
            start_ms: 2000,
            end_ms: 3000,
            zoom_in_ms: 200,
            zoom_out_ms: 200,
            target_scale: 2.5,
            anchor: FramePoint { x: 100, y: 100 },
            easing: Easing::Smooth,
            easing_out: Easing::Smooth,
            layer: 1,
            cam_action: None,
            follow_cursor: false,
        },
    ];
    let mut c = Camera {
        cx: 0.0,
        cy: 0.0,
        scale: 1.0,
    };
    for t in (0..2400).step_by(16) {
        c = s.step(t, STEP, FramePoint { x: 700, y: 500 }, &regions, &cfg);
    }
    assert!(
        (c.scale - 2.5).abs() < 0.05,
        "expected to reach B's target scale 2.5: {}",
        c.scale
    );
}
