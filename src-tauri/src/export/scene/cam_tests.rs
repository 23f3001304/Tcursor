// Tests for the camera-pose conversions in export::scene::mod (`rect_from_center` /
// `override_camera`), split out of mod_tests.rs so both stay under the size limit.
use super::*;

// `rect_from_center` converts a sampled CamPose into the camera panel's RectF. `aspect`
// is the panel's w/h, so the keyframed rect keeps the shape the static panel resolved to.
#[test]
fn rect_from_center_is_squared_and_centered() {
    let p = CamPose { x: 0.5, y: 0.5, size: 0.3 };
    let (ow, oh) = (1920.0, 1080.0);
    let r = rect_from_center(p, ow, oh, 1.0);
    let h = 0.3 * oh;
    assert!((r.h - h).abs() < 1e-3);
    assert!((r.w - h).abs() < 1e-3, "aspect 1.0: w must equal h");
    assert!((r.x + r.w / 2.0 - p.x * ow).abs() < 1e-3, "center_x");
    assert!((r.y + r.h / 2.0 - p.y * oh).abs() < 1e-3, "center_y");
}

// The Wide-panel bug: one camera_moves keyframe used to square a 16:9 panel for the whole
// clip, because width was copied from the height-derived side.
#[test]
fn rect_from_center_keeps_a_wide_panel_wide() {
    let (ow, oh, a) = (1920.0, 1080.0, 16.0 / 9.0);
    let r = rect_from_center(CamPose { x: 0.5, y: 0.5, size: 0.25 }, ow, oh, a);
    assert!((r.h - 0.25 * oh).abs() < 1e-3, "height still comes from `size` alone");
    assert!((r.w / r.h - a).abs() < 1e-3, "want 16:9, got {}", r.w / r.h);
    assert!((r.x + r.w / 2.0 - 0.5 * ow).abs() < 1e-3, "still centered on the pose in x");
    assert!((r.y + r.h / 2.0 - 0.5 * oh).abs() < 1e-3, "still centered on the pose in y");
}

// `static_cam_pose` -> `rect_from_center` must round-trip a Wide panel exactly: the pose
// carries height only, so the aspect the renderer feeds back in has to restore the width.
#[test]
fn static_cam_pose_round_trips_a_wide_rect() {
    let (ow, oh) = (1920.0, 1080.0);
    let rect = RectF { x: 100.0, y: 700.0, w: 384.0, h: 216.0 }; // 16:9 bubble
    let p = crate::export::camera::static_cam_pose(rect, ow, oh);
    let back = rect_from_center(p, ow, oh, rect.w / rect.h);
    assert!((back.x - rect.x).abs() < 1e-3 && (back.y - rect.y).abs() < 1e-3);
    assert!((back.w - rect.w).abs() < 1e-3 && (back.h - rect.h).abs() < 1e-3);
}

// override_camera must scale radius (and ring) by the height ratio so a circle panel
// (radius == min(w,h)/2 at its STATIC size) stays a true circle after a keyframe
// grows/shrinks it, instead of leaving radius at the pre-override value.
#[test]
fn override_camera_keeps_circle_round_after_resize() {
    // Static circle panel: 200x200, radius 100 (min(w,h)/2).
    let panel = Panel { rect: RectF { x: 50.0, y: 50.0, w: 200.0, h: 200.0 }, radius: 100.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] };
    let p = CamPose { x: 0.5, y: 0.5, size: 0.1 }; // shrinks to 10% of oh=1000 -> 100x100
    let out = override_camera(panel, p, 1000.0, 1000.0, 1.0);
    assert_eq!(out.rect.w, 100.0);
    assert_eq!(out.rect.h, 100.0);
    // radius must scale with the height ratio (100/200 = 0.5) -> 50, i.e. still min(w,h)/2.
    assert!((out.radius - 50.0).abs() < 1e-3);
    assert!((out.radius - out.rect.w.min(out.rect.h) / 2.0).abs() < 1e-3, "must still read as a true circle");
}

#[test]
fn override_camera_grows_radius_when_panel_grows() {
    let panel = Panel { rect: RectF { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, radius: 50.0, alpha: 1.0, ring_px: 4.0, ring_color: [1, 2, 3] };
    let p = CamPose { x: 0.5, y: 0.5, size: 0.4 }; // grows to 40% of oh=1000 -> 400x400 (4x)
    let out = override_camera(panel, p, 1000.0, 1000.0, 1.0);
    assert!((out.radius - 200.0).abs() < 1e-3); // 50 * 4
    assert!((out.ring_px - 16.0).abs() < 1e-3);  // ring scales the same way (4 * 4)
    assert_eq!(out.ring_color, [1, 2, 3]);       // color untouched
    assert_eq!(out.alpha, 1.0);                  // alpha untouched (..panel)
}

#[test]
fn override_camera_matches_rect_from_center_position() {
    // The rect itself must be identical to plain rect_from_center (only radius/ring differ).
    let panel = Panel { rect: RectF { x: 10.0, y: 10.0, w: 50.0, h: 50.0 }, radius: 25.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] };
    let p = CamPose { x: 0.3, y: 0.7, size: 0.2 };
    let (ow, oh) = (1920.0, 1080.0);
    let plain = rect_from_center(p, ow, oh, 1.0);
    let out = override_camera(panel, p, ow, oh, 1.0);
    assert_eq!(out.rect, plain);
}
