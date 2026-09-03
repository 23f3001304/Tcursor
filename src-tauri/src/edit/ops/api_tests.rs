// Tests for edit::ops::api, split into their own file so api.rs stays under the size limit.
use super::*;
use crate::edit::model::{EditDoc, Trim};

fn empty() -> EditDoc { EditDoc::default() }

#[test]
fn add_zoom_clamps_end_to_trim_duration() {
    // Adding a zoom near the end of the clip (playhead at 800ms of a 1000ms clip) must not
    // let end_ms overflow past the clip - that overflow was what made the timeline pill (and
    // the camera's zoom-out ramp, which never got to run) go "out of bounds".
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 800, dur_ms: 2000 });
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms), (800, 1000));
}

#[test]
fn update_zoom_clamps_end_to_trim_duration() {
    let mut doc = empty(); doc.trim.out_ms = 1000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 500 });
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id, start_ms: None, end_ms: Some(5000), scale: None,
        target: None, easing: None, zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!(doc.zooms[0].end_ms, 1000);
}

#[test]
fn add_zoom_bounds_to_clip_ms_not_the_trim_point() {
    // clip_ms is the TRUE clip length; trim.out_ms (10_000, from an earlier trim) is just where
    // playback currently ends - it must not collapse a region placed later in the full clip.
    let mut doc = empty();
    doc.clip_ms = 60_000;
    doc.trim.out_ms = 10_000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 30_000, dur_ms: 2_000 });
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms), (30_000, 32_000));
}

#[test]
fn add_zoom_auto_assigns_a_free_layer() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 1000 });
    apply(&mut doc, EditOp::AddZoom { at_ms: 500, dur_ms: 1000 }); // overlaps the first
    assert_eq!(doc.zooms[0].layer, 0);
    assert_eq!(doc.zooms[1].layer, 1);
}

/// A dedicated op so "clear back to inherit" is expressible - `UpdateZoom`'s
/// None-means-unchanged convention cannot say that without an `Option<Option<_>>`.
#[test]
fn set_zoom_cam_action_sets_then_clears() {
    use crate::settings::model::CamZoomAction;
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 500 });
    let id = doc.zooms[0].id.clone();
    assert_eq!(doc.zooms[0].cam_action, None, "new zooms inherit the global default");

    apply(&mut doc, EditOp::SetZoomCamAction { id: id.clone(), action: Some(CamZoomAction::Stay) });
    assert_eq!(doc.zooms[0].cam_action, Some(CamZoomAction::Stay));

    apply(&mut doc, EditOp::SetZoomCamAction { id, action: None });
    assert_eq!(doc.zooms[0].cam_action, None, "must be able to clear back to inherit");
}

#[test]
fn update_zoom_sets_layer_and_validates_easing() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 500 });
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id: id.clone(), start_ms: None, end_ms: None, scale: None,
        target: None, easing: Some("spring".into()), zoom_in_ms: None, zoom_out_ms: None, layer: Some(2) });
    assert_eq!((doc.zooms[0].layer, doc.zooms[0].easing.as_str()), (2, "spring"));
    // Unknown easing falls back to "smooth", exactly like UpdateLayoutSeg/UpdateCameraMove.
    apply(&mut doc, EditOp::UpdateZoom { id, start_ms: None, end_ms: None, scale: None,
        target: None, easing: Some("bogus".into()), zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!(doc.zooms[0].easing, "smooth");
}

#[test]
fn add_zoom_appends_and_yields_distinct_ids() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 500, dur_ms: 1000 });
    let z = &doc.zooms[0];
    assert_eq!((z.start_ms, z.end_ms, z.scale, z.easing.as_str()), (500, 1500, 2.0, "smooth"));
    apply(&mut doc, EditOp::AddZoom { at_ms: 200, dur_ms: 100 });
    assert_ne!(doc.zooms[0].id, doc.zooms[1].id);
    assert!(doc.zooms[0].id.starts_with('z'));
}

#[test]
fn update_zoom_changes_fields_and_ignores_unknown() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 500 });
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id: id.clone(), start_ms: Some(100), end_ms: None,
        scale: None, target: None, easing: None, zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms, doc.zooms[0].scale), (100, 500, 2.0));
    apply(&mut doc, EditOp::UpdateZoom { id: "z999".into(), start_ms: Some(9999), end_ms: None,
        scale: None, target: None, easing: None, zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!(doc.zooms[0].start_ms, 100);
}

#[test]
fn update_zoom_sets_durations() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 1000 });
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id, start_ms: None, end_ms: None, scale: None, target: None, easing: None, zoom_in_ms: Some(120), zoom_out_ms: Some(640), layer: None });
    assert_eq!((doc.zooms[0].zoom_in_ms, doc.zooms[0].zoom_out_ms), (120, 640));
}

#[test]
fn remove_zoom_drops_by_id() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 100 });
    apply(&mut doc, EditOp::AddZoom { at_ms: 200, dur_ms: 100 });
    let id = doc.zooms[0].id.clone(); apply(&mut doc, EditOp::RemoveZoom { id });
    assert_eq!(doc.zooms.len(), 1);
}

#[test]
fn set_trim_replaces_trim() {
    let mut doc = empty();
    apply(&mut doc, EditOp::SetTrim { in_ms: 200, out_ms: 8000 });
    assert_eq!(doc.trim, Trim { in_ms: 200, out_ms: 8000 });
}

#[test]
fn set_aspect_replaces_aspect() {
    use crate::export::types::Aspect;
    let mut doc = empty();
    assert_eq!(doc.aspect, Aspect::Source);
    apply(&mut doc, EditOp::SetAspect { aspect: Aspect::Vertical9x16 });
    assert_eq!(doc.aspect, Aspect::Vertical9x16);
}

#[test]
fn add_zoom_full_uses_given_scale() {
    let mut doc = empty();
    apply(&mut doc, EditOp::AddZoomFull { at_ms: 100, dur_ms: 500, scale: 3.0 });
    let z = &doc.zooms[0];
    assert_eq!((z.start_ms, z.end_ms, z.scale), (100, 600, 3.0));
    assert!(z.id.starts_with('z'));
}

#[test]
fn add_layout_seg_appends_clamped_with_default_feel() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 800, dur_ms: 2000, layout: "camera".into(), transition_out_ms: None, easing_out: None });
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!((s.start_ms, s.end_ms, s.layout.as_str()), (800, 1000, "camera")); // end clamped to clip
    assert_eq!((s.transition_ms, s.easing.as_str()), (350, "smooth"));
    assert!(s.id.starts_with('l'));
}

#[test]
fn add_layout_seg_rejects_unknown_preset() {
    let mut doc = empty(); doc.trim.out_ms = 5000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 0, dur_ms: 1000, layout: "bogus".into(), transition_out_ms: None, easing_out: None });
    assert_eq!(doc.layout[doc.layout.len() - 1].layout, "screen"); // unknown -> screen
}

#[test]
fn update_layout_seg_patches_feel_and_preset() {
    let mut doc = empty(); doc.trim.out_ms = 5000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 0, dur_ms: 1000, layout: "screen".into(), transition_out_ms: None, easing_out: None });
    let id = doc.layout[doc.layout.len() - 1].id.clone();
    apply(&mut doc, EditOp::UpdateLayoutSeg { id: id.clone(), start_ms: None, end_ms: None,
        layout: Some("presenter".into()), transition_ms: Some(120), easing: Some("spring".into()),
        transition_out_ms: None, easing_out: None });
    let s = doc.layout.iter().find(|s| s.id == id).unwrap();
    assert_eq!((s.layout.as_str(), s.transition_ms, s.easing.as_str()), ("presenter", 120, "spring"));
}

#[test]
fn remove_layout_seg_drops_by_id() {
    let mut doc = empty(); doc.trim.out_ms = 5000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 0, dur_ms: 500, layout: "camera".into(), transition_out_ms: None, easing_out: None });
    let id = doc.layout[doc.layout.len() - 1].id.clone();
    let before = doc.layout.len();
    apply(&mut doc, EditOp::RemoveLayoutSeg { id });
    assert_eq!(doc.layout.len(), before - 1);
}

// Camera-move + M5 clamp op tests live in their own files - api_tests.rs was at the 200-line budget.
#[path = "api_camera_tests.rs"]
mod camera_tests;
#[path = "api_clamp_tests.rs"]
mod clamp_tests;
