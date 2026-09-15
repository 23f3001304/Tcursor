use super::*;

#[test]
fn rect_from_center_is_squared_and_centered() {
    let p = CamPose {
        x: 0.5,
        y: 0.5,
        size: 0.3,
        round: None,
    };
    let (ow, oh) = (1920.0, 1080.0);
    let r = rect_from_center(p, ow, oh, 1.0);
    let h = 0.3 * oh;
    assert!((r.h - h).abs() < 1e-3);
    assert!((r.w - h).abs() < 1e-3, "aspect 1.0: w must equal h");
    assert!((r.x + r.w / 2.0 - p.x * ow).abs() < 1e-3, "center_x");
    assert!((r.y + r.h / 2.0 - p.y * oh).abs() < 1e-3, "center_y");
}

#[test]
fn rect_from_center_keeps_a_wide_panel_wide() {
    let (ow, oh, a) = (1920.0, 1080.0, 16.0 / 9.0);
    let r = rect_from_center(
        CamPose {
            x: 0.5,
            y: 0.5,
            size: 0.25,
            round: None,
        },
        ow,
        oh,
        a,
    );
    assert!(
        (r.h - 0.25 * oh).abs() < 1e-3,
        "height still comes from `size` alone"
    );
    assert!((r.w / r.h - a).abs() < 1e-3, "want 16:9, got {}", r.w / r.h);
    assert!(
        (r.x + r.w / 2.0 - 0.5 * ow).abs() < 1e-3,
        "still centered on the pose in x"
    );
    assert!(
        (r.y + r.h / 2.0 - 0.5 * oh).abs() < 1e-3,
        "still centered on the pose in y"
    );
}

#[test]
fn static_cam_pose_round_trips_a_wide_rect() {
    let (ow, oh) = (1920.0, 1080.0);
    let rect = RectF {
        x: 100.0,
        y: 700.0,
        w: 384.0,
        h: 216.0,
    };
    let panel = Panel {
        rect,
        radius: 108.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    };
    let p = crate::export::camera::static_cam_pose(&panel, ow, oh);
    let back = rect_from_center(p, ow, oh, rect.w / rect.h);
    assert!((back.x - rect.x).abs() < 1e-3 && (back.y - rect.y).abs() < 1e-3);
    assert!((back.w - rect.w).abs() < 1e-3 && (back.h - rect.h).abs() < 1e-3);
    assert_eq!(
        p.round,
        Some(0.5),
        "the live pose carries the panel's shape as a fraction of its short side"
    );
}

#[test]
fn override_camera_keeps_circle_round_after_resize() {
    let panel = Panel {
        rect: RectF {
            x: 50.0,
            y: 50.0,
            w: 200.0,
            h: 200.0,
        },
        radius: 100.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    };
    let p = CamPose {
        x: 0.5,
        y: 0.5,
        size: 0.1,
        round: None,
    };
    let out = override_camera(panel, p, 1000.0, 1000.0, 1.0);
    assert_eq!(out.rect.w, 100.0);
    assert_eq!(out.rect.h, 100.0);
    assert!((out.radius - 50.0).abs() < 1e-3);
    assert!(
        (out.radius - out.rect.w.min(out.rect.h) / 2.0).abs() < 1e-3,
        "must still read as a true circle"
    );
}

#[test]
fn override_camera_grows_radius_when_panel_grows() {
    let panel = Panel {
        rect: RectF {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        radius: 50.0,
        alpha: 1.0,
        ring_px: 4.0,
        ring_color: [1, 2, 3],
    };
    let p = CamPose {
        x: 0.5,
        y: 0.5,
        size: 0.4,
        round: None,
    };
    let out = override_camera(panel, p, 1000.0, 1000.0, 1.0);
    assert!((out.radius - 200.0).abs() < 1e-3);
    assert!((out.ring_px - 16.0).abs() < 1e-3);
    assert_eq!(out.ring_color, [1, 2, 3]);
    assert_eq!(out.alpha, 1.0);
}

#[test]
fn override_camera_matches_rect_from_center_position() {
    let panel = Panel {
        rect: RectF {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 50.0,
        },
        radius: 25.0,
        alpha: 1.0,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    };
    let p = CamPose {
        x: 0.3,
        y: 0.7,
        size: 0.2,
        round: None,
    };
    let (ow, oh) = (1920.0, 1080.0);
    let plain = rect_from_center(p, ow, oh, 1.0);
    let out = override_camera(panel, p, ow, oh, 1.0);
    assert_eq!(out.rect, plain);
}

#[test]
fn override_camera_takes_the_poses_own_shape_when_it_has_one() {
    let panel = Panel {
        rect: RectF {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        radius: 12.0,
        alpha: 1.0,
        ring_px: 4.0,
        ring_color: [1, 2, 3],
    };
    let circle = override_camera(
        panel,
        CamPose {
            x: 0.5,
            y: 0.5,
            size: 0.4,
            round: Some(0.5),
        },
        1000.0,
        1000.0,
        1.0,
    );
    assert!(
        (circle.radius - 200.0).abs() < 1e-3,
        "half the 400 px short side: {}",
        circle.radius
    );
    let rect = override_camera(
        panel,
        CamPose {
            x: 0.5,
            y: 0.5,
            size: 0.4,
            round: Some(0.0),
        },
        1000.0,
        1000.0,
        1.0,
    );
    assert_eq!(rect.radius, 0.0);
    let wide = override_camera(
        panel,
        CamPose {
            x: 0.5,
            y: 0.5,
            size: 0.2,
            round: Some(0.25),
        },
        1000.0,
        1000.0,
        16.0 / 9.0,
    );
    assert!(
        (wide.radius - 50.0).abs() < 1e-3,
        "a quarter of the SHORT side (h=200), not the width: {}",
        wide.radius
    );
    assert!(
        (wide.ring_px - 8.0).abs() < 1e-3,
        "the ring still scales by the height ratio"
    );
    let inherit = override_camera(
        panel,
        CamPose {
            x: 0.5,
            y: 0.5,
            size: 0.4,
            round: None,
        },
        1000.0,
        1000.0,
        1.0,
    );
    assert!(
        (inherit.radius - 48.0).abs() < 1e-3,
        "no shape: the static 12 px scaled 4x"
    );
}
