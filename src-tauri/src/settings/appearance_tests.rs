use super::*;

#[test]
fn defaults_reproduce_today_values() {
    let a = AppearanceSettings::default();
    assert_eq!(a.screen.pad, 0.03125);
    assert_eq!(a.screen.cam_shape, CamShape::Circle);
    assert_eq!(a.screen.cam_corner, CamCorner::BottomLeft);
    assert_eq!(a.camera.cam_size, 0.889);
    assert_eq!(a.camera.cam_shape, CamShape::Rounded);
    assert_eq!(a.screen.cam_aspect, CamAspect::Square);
    assert_eq!(a.camera.cam_aspect, CamAspect::Square);
    assert_eq!(a.screen.cam_ring, None);
    assert_eq!(a.camera.cam_ring, None);
}
#[test]
fn for_id_maps_each_mode() {
    let a = AppearanceSettings::default();
    assert_eq!(*a.for_id(LayoutId::Screen), a.screen);
    assert_eq!(*a.for_id(LayoutId::ScreenOnly), a.screen_only);
    assert_eq!(*a.for_id(LayoutId::Camera), a.camera);
    assert_eq!(*a.for_id(LayoutId::CameraOnly), a.camera_only);
    assert_eq!(*a.for_id(LayoutId::Presenter), a.presenter);
}
#[test]
fn round_trip_and_partial_json_fill_defaults() {
    let a = AppearanceSettings::default();
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.contains("\"cam_shape\":\"circle\""));
    assert!(json.contains("\"cam_corner\":\"bottom_left\""));
    let back: AppearanceSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, a);
    let p: AppearanceSettings = serde_json::from_str("{\"screen\":{\"pad\":0.05}}").unwrap();
    assert_eq!(p.screen.pad, 0.05);
    assert_eq!(p.screen.cam_size, ModeAppearance::default().cam_size);
    assert_eq!(p.camera, AppearanceSettings::default().camera);
}
#[test]
fn layout_for_default_screen_is_today_px() {
    let l = layout_for(&AppearanceSettings::default().screen, 3840, 2160);
    assert_eq!(l.pad_px, 120);
    assert_eq!(l.screen_scale, 1.0);
    assert!((l.screen_radius_px - 34.56).abs() < 0.01);
}
#[test]
fn overlay_for_default_screen_is_today_bubble() {
    use crate::export::types::{OverlayPos, OverlayShape};
    let o = overlay_for(&AppearanceSettings::default().screen, 3840, 2160, true);
    assert_eq!(o.size_px, 420);
    assert_eq!(o.width_px, 420);
    assert_eq!((o.margin_x_px, o.margin_y_px), (80, 80));
    assert!(matches!(o.shape, OverlayShape::Circle));
    assert!(matches!(o.pos, OverlayPos::BottomLeft));
    assert_eq!(o.ring_px, 0);
    assert_eq!(o.ring_color, [0, 0, 0]);
}
#[test]
fn overlay_for_default_camera_is_rounded_1920() {
    use crate::export::types::OverlayShape;
    let o = overlay_for(&AppearanceSettings::default().camera, 3840, 2160, true);
    assert_eq!(o.size_px, 1920);
    assert_eq!(o.width_px, 1920);
    assert!(matches!(o.shape, OverlayShape::Rounded { .. }));
}
#[test]
fn overlay_for_wide_aspect_scales_width_only() {
    let mut ma = AppearanceSettings::default().screen;
    ma.cam_aspect = CamAspect::Wide;
    let o = overlay_for(&ma, 3840, 2160, true);
    assert_eq!(o.size_px, 420);
    assert_eq!(o.width_px, (420.0f32 * 16.0 / 9.0).round() as u32);
}
#[test]
fn overlay_for_ring_scales_from_min_side() {
    let mut ma = AppearanceSettings::default().screen;
    ma.cam_ring = Some(CamRing {
        width: 0.05,
        color: [10, 20, 30],
    });
    let o = overlay_for(&ma, 3840, 2160, true);
    assert_eq!(o.ring_px, (0.05 * 420.0f32).round() as u32);
    assert_eq!(o.ring_color, [10, 20, 30]);
}
#[test]
fn overlay_for_no_ring_is_zero() {
    let o = overlay_for(&AppearanceSettings::default().screen, 3840, 2160, true);
    assert_eq!(o.ring_px, 0);
    assert_eq!(o.ring_color, [0, 0, 0]);
}
