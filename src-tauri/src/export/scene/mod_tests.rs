// Tests for export::scene::mod, split out so mod.rs stays under the size limit.
use super::*;
use crate::export::coordmap::inset_rect;
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings, CamAspect, CamCorner, CamRing, CamShape};

fn mode(id: LayoutId, sw: u32, sh: u32) -> Scene {
    let a = AppearanceSettings::default();
    let ma = a.for_id(id);
    resolve(id, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), sw, sh)
}

#[test]
fn screen_default_matches_today_inset_and_bubble() {
    let s = mode(LayoutId::Screen, 1920, 1080);
    let lay = layout_for(&AppearanceSettings::default().screen, 3840, 2160);
    let (ix, iy, iw, ih) = inset_rect(1920, 1080, &lay);
    assert_eq!(s.screen.rect, RectF { x: ix as f32, y: iy as f32, w: iw as f32, h: ih as f32 });
    assert_eq!(s.camera.rect.w, 420.0);
    assert_eq!(s.camera.rect.x, 80.0);
    assert!((s.camera.rect.y - (2160.0 - 420.0 - 80.0)).abs() < 1.0);
    assert!((s.camera.radius - 210.0).abs() < 1e-3);   // circle
    assert_eq!((s.screen.alpha, s.camera.alpha), (1.0, 1.0));
}
#[test]
fn camera_default_is_big_centered_rounded_square() {
    let s = mode(LayoutId::Camera, 1920, 1080);
    assert!((s.camera.rect.w - 1920.0).abs() < 1.0);
    assert!((s.camera.rect.x - (3840.0 - 1920.0) / 2.0).abs() < 1.0);
    assert!((s.camera.radius - 0.04 * 1920.0).abs() < 1.0); // 76.8
    assert!(s.camera.rect.w * s.camera.rect.h > s.screen.rect.w * s.screen.rect.h);
}
#[test]
fn corner_and_shape_knobs_apply() {
    let mut a = AppearanceSettings::default();
    a.screen.cam_corner = CamCorner::TopRight;
    a.screen.cam_shape = CamShape::Rect;
    let ma = a.for_id(LayoutId::Screen);
    let s = resolve(LayoutId::Screen, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), 1920, 1080);
    assert_eq!(s.camera.rect.x, 3840.0 - 420.0 - 80.0); // right edge
    assert_eq!(s.camera.rect.y, 80.0);                  // top edge
    assert_eq!(s.camera.radius, 0.0);                   // rect -> no rounding
}
// Task 9 Part A: Wide aspect widens the bubble (width_px) but keeps height (size_px)
// and the right-anchored corner math intact.
#[test]
fn wide_aspect_widens_bubble_and_keeps_right_anchor() {
    let mut a = AppearanceSettings::default();
    a.screen.cam_aspect = CamAspect::Wide;
    let ma = a.for_id(LayoutId::Screen);
    let ov = overlay_for(ma, 3840, 2160, true);
    let expect_w = (420.0f32 * 16.0 / 9.0).round(); // 747
    assert_eq!(ov.width_px as f32, expect_w);
    assert_eq!(ov.size_px, 420); // height unchanged
    let s = resolve(LayoutId::Screen, &layout_for(ma, 3840, 2160), &ov, 1920, 1080);
    assert_eq!(s.camera.rect.w, expect_w);
    assert_eq!(s.camera.rect.h, 420.0);
    assert_eq!(s.camera.rect.x, 80.0); // BottomLeft: x unaffected by width
    // Right-anchored corner must use width_px, not size_px, for its x position.
    let mut b = a.clone();
    b.screen.cam_corner = CamCorner::BottomRight;
    let mb = b.for_id(LayoutId::Screen);
    let ovb = overlay_for(mb, 3840, 2160, true);
    let sb = resolve(LayoutId::Screen, &layout_for(mb, 3840, 2160), &ovb, 1920, 1080);
    assert_eq!(sb.camera.rect.x, 3840.0 - expect_w - 80.0);
}
// Task 9 Part B: ring flows from ModeAppearance.cam_ring -> overlay_for -> resolve's camera
// Panel, and never touches the screen panel.
#[test]
fn cam_ring_flows_onto_camera_panel_only() {
    let mut a = AppearanceSettings::default();
    a.screen.cam_ring = Some(CamRing { width: 0.05, color: [11, 22, 33] });
    let ma = a.for_id(LayoutId::Screen);
    let ov = overlay_for(ma, 3840, 2160, true);
    let s = resolve(LayoutId::Screen, &layout_for(ma, 3840, 2160), &ov, 1920, 1080);
    assert_eq!(s.camera.ring_px, ov.ring_px as f32);
    assert!(s.camera.ring_px > 0.0);
    assert_eq!(s.camera.ring_color, [11, 22, 33]);
    assert_eq!(s.screen.ring_px, 0.0); // screen panel never carries a ring
}
#[test]
fn no_cam_ring_leaves_camera_panel_ring_zero() {
    let s = mode(LayoutId::Screen, 1920, 1080);
    assert_eq!(s.camera.ring_px, 0.0);
    assert_eq!(s.camera.ring_color, [0, 0, 0]);
}
// Square default: width_px == size_px, so bubble stays exactly as before (byte-identical).
#[test]
fn square_aspect_bubble_rect_matches_pre_task9_shape() {
    let s = mode(LayoutId::Screen, 1920, 1080);
    assert_eq!(s.camera.rect.w, s.camera.rect.h);
    assert_eq!(s.camera.rect.w, 420.0);
}
// Big-camera / centered-square modes ignore width_px entirely - aspect is bubble-only.
#[test]
fn wide_aspect_does_not_affect_big_camera_modes() {
    let mut a = AppearanceSettings::default();
    a.camera.cam_aspect = CamAspect::Wide;
    let ma = a.for_id(LayoutId::Camera);
    let s = resolve(LayoutId::Camera, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), 1920, 1080);
    assert_eq!(s.camera.rect.w, s.camera.rect.h); // still square
}
#[test]
fn camera_only_disables_screen() {
    let s = mode(LayoutId::CameraOnly, 1920, 1080);
    assert_eq!(s.screen.alpha, 0.0);
    assert_eq!(s.camera.alpha, 1.0);
}
#[test]
fn camera_default_pip_screen_keeps_today_inset() {
    let s = mode(LayoutId::Camera, 1920, 1080);
    // PiP screen stays inset 80px from the bottom-left (today's behavior), not flush.
    assert!((s.screen.rect.x - 80.0).abs() < 1.0);
    let small_h = s.screen.rect.h;
    assert!((s.screen.rect.y - (2160.0 - small_h - 80.0)).abs() < 1.0);
}
#[test]
fn lerp_midpoint_is_between() {
    let a = mode(LayoutId::Screen, 1920, 1080);
    let b = mode(LayoutId::Camera, 1920, 1080);
    let m = Scene::lerp(&a, &b, 0.5);
    assert!((m.screen.rect.w - (a.screen.rect.w + b.screen.rect.w) / 2.0).abs() < 1e-3);
}
#[test]
fn shrink_is_identity_at_no_zoom_and_min_at_full() {
    let p = Panel { rect: RectF { x: 100.0, y: 100.0, w: 200.0, h: 200.0 }, radius: 100.0, alpha: 1.0, ring_px: 20.0, ring_color: [255, 0, 0] };
    let none = shrink_camera(p, 1.0, 2.2, 0.6);
    assert_eq!(none.rect.w, 200.0);
    assert_eq!((none.rect.x, none.rect.y), (100.0, 100.0));
    assert_eq!(none.ring_px, 20.0);
    let full = shrink_camera(p, 2.2, 2.2, 0.6);
    assert!((full.rect.w - 120.0).abs() < 0.5);
    assert!((full.radius - 60.0).abs() < 0.5);
    assert!((full.rect.x + full.rect.w / 2.0 - 200.0).abs() < 0.5);
    assert!((full.ring_px - 12.0).abs() < 0.5); // ring scales in lockstep with the panel (0.6x)
    assert_eq!(full.ring_color, [255, 0, 0]);   // color untouched by shrink
}

// `rect_from_center` / `override_camera` have their own file: scene/cam_tests.rs.

// Task 4: byte-identical guard - an empty camera_moves track leaves the resolved static
// scene camera rect (from `resolve`/`overlay_for`) completely untouched. `FrameRenderer::
// step_camera` only calls `rect_from_center` inside `if let Some(p) = cam_moves.sample(..)`,
// so this proves the override math itself is never invoked when there is nothing to sample -
// the actual wiring (None for `&[]`, regardless of `live` - the empty-track check runs first,
// before the span test) is exercised by export::camera::moves_tests::empty_track_samples_to_none.
#[test]
fn empty_track_leaves_static_scene_camera_rect_unchanged() {
    let before = mode(LayoutId::Screen, 1920, 1080);
    let track = crate::export::camera::moves::CameraMoveTrack::from_doc(&[]);
    assert_eq!(track.sample(0, Some(CamPose { x: 0.5, y: 0.5, size: 0.2 })), None);
    let after = mode(LayoutId::Screen, 1920, 1080);
    assert_eq!(after.camera.rect, before.camera.rect, "static overlay_for rect must be untouched");
}
