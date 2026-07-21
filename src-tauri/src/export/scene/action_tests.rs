// Tests for `apply_cam_zoom_action` + the resolved global default. Split out so
// scene/mod_tests.rs stays under the size limit.
use super::*;
use crate::export::types::{Easing, FramePoint};
use crate::settings::model::ZoomSettings;

fn region(start: u32, end: u32, layer: u32, action: Option<CamZoomAction>) -> ZoomRegion {
    ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: 300, zoom_out_ms: 300, target_scale: 2.0,
        anchor: FramePoint { x: 0, y: 0 }, easing: Easing::Smooth, layer, cam_action: action }
}

fn panel() -> Panel {
    Panel { rect: RectF { x: 100.0, y: 100.0, w: 200.0, h: 200.0 }, radius: 100.0, alpha: 1.0,
        ring_px: 20.0, ring_color: [255, 0, 0] }
}

// THE byte-identity guard: `Shrink` must delegate to `shrink_camera`, so a default-resolved
// action reproduces the pre-action behavior at EVERY sampled scale, not just the endpoints.
#[test]
fn shrink_action_is_identical_to_shrink_camera() {
    for &s in &[1.0f32, 1.3, 1.7, 2.2] {
        let a = apply_cam_zoom_action(panel(), CamZoomAction::Shrink { to: 0.62 }, s, 2.2);
        let b = shrink_camera(panel(), s, 2.2, 0.62);
        assert_eq!(a, b, "scale {}", s);
    }
}

#[test]
fn stay_is_the_identity_at_every_scale() {
    for &s in &[1.0f32, 1.5, 2.2] {
        assert_eq!(apply_cam_zoom_action(panel(), CamZoomAction::Stay, s, 2.2), panel(), "scale {}", s);
    }
}

#[test]
fn hide_fades_alpha_out_as_the_zoom_deepens() {
    let none = apply_cam_zoom_action(panel(), CamZoomAction::Hide, 1.0, 2.2);
    assert_eq!(none.alpha, 1.0); // no zoom -> untouched
    let full = apply_cam_zoom_action(panel(), CamZoomAction::Hide, 2.2, 2.2);
    assert!(full.alpha.abs() < 1e-6, "full zoom -> gone, got {}", full.alpha);
    let mid = apply_cam_zoom_action(panel(), CamZoomAction::Hide, 1.6, 2.2);
    assert!(mid.alpha > 0.0 && mid.alpha < 1.0, "mid zoom -> partial, got {}", mid.alpha);
    // Hide must ONLY touch alpha - never move or resize the panel.
    assert_eq!(full.rect, panel().rect);
    assert_eq!(full.radius, panel().radius);
    assert_eq!(full.ring_px, panel().ring_px);
}

// Back-compat: settings written before `cam_zoom_default` existed must resolve to EXACTLY
// today's behavior (shrink to the legacy 0.62 floor, or Stay when the toggle is off).
#[test]
fn resolved_default_derives_from_the_legacy_shrink_fields() {
    let s = ZoomSettings::default();
    assert_eq!(s.cam_zoom_default, None);
    assert_eq!(s.resolved_cam_action(), CamZoomAction::Shrink { to: 0.62 });

    let off = ZoomSettings { camera_shrink: false, ..ZoomSettings::default() };
    assert_eq!(off.resolved_cam_action(), CamZoomAction::Stay);

    let explicit = ZoomSettings { cam_zoom_default: Some(CamZoomAction::Hide), ..ZoomSettings::default() };
    assert_eq!(explicit.resolved_cam_action(), CamZoomAction::Hide, "explicit default beats legacy fields");
}

#[test]
fn cam_action_falls_back_to_the_global_default() {
    let z = ZoomSettings::default();
    assert_eq!(cam_action_at(&[], &z, 500), CamZoomAction::Shrink { to: 0.62 }, "no regions");
    let rs = vec![region(0, 1000, 0, None)];
    assert_eq!(cam_action_at(&rs, &z, 500), CamZoomAction::Shrink { to: 0.62 }, "region inherits");
    assert_eq!(cam_action_at(&rs, &z, 5000), CamZoomAction::Shrink { to: 0.62 }, "outside every region");
}

#[test]
fn per_zoom_override_beats_the_global_default_but_only_inside_the_region() {
    let z = ZoomSettings::default();
    let rs = vec![region(0, 1000, 0, Some(CamZoomAction::Stay))];
    assert_eq!(cam_action_at(&rs, &z, 500), CamZoomAction::Stay);
    assert_eq!(cam_action_at(&rs, &z, 2000), CamZoomAction::Shrink { to: 0.62 });
}

// Overlap resolution must agree with `CameraSim::step`, which also lets the highest layer win -
// otherwise the webcam would follow one zoom while the framing followed another.
#[test]
fn highest_layer_region_supplies_the_action_when_zooms_overlap() {
    let z = ZoomSettings::default();
    let rs = vec![
        region(0, 2000, 0, Some(CamZoomAction::Stay)),
        region(1000, 3000, 1, Some(CamZoomAction::Hide)),
    ];
    assert_eq!(cam_action_at(&rs, &z, 1500), CamZoomAction::Hide, "layer 1 wins the overlap");
    assert_eq!(cam_action_at(&rs, &z, 500), CamZoomAction::Stay, "only layer 0 is active here");
}

// Old settings JSON (no `cam_zoom_default` key) must deserialize and still resolve to today's look.
#[test]
fn settings_json_without_cam_zoom_default_loads_and_resolves_to_shrink() {
    let s: ZoomSettings = serde_json::from_str(r#"{"camera_shrink":true,"camera_shrink_min":0.62}"#).unwrap();
    assert_eq!(s.cam_zoom_default, None);
    assert_eq!(s.resolved_cam_action(), CamZoomAction::Shrink { to: 0.62 });
}
