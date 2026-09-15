use super::*;
use crate::edit::model::{EditDoc, Trim};

fn empty() -> EditDoc {
    EditDoc::default()
}

#[test]
fn add_zoom_clamps_end_to_trim_duration() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 800,
            dur_ms: 2000,
        },
    );
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms), (800, 1000));
}

#[test]
fn add_zoom_bounds_to_clip_ms_not_the_trim_point() {
    let mut doc = empty();
    doc.clip_ms = 60_000;
    doc.trim.out_ms = 10_000;
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 30_000,
            dur_ms: 2_000,
        },
    );
    assert_eq!(
        (doc.zooms[0].start_ms, doc.zooms[0].end_ms),
        (30_000, 32_000)
    );
}

#[test]
fn add_zoom_auto_assigns_a_free_layer() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 500,
            dur_ms: 1000,
        },
    );
    assert_eq!(doc.zooms[0].layer, 0);
    assert_eq!(doc.zooms[1].layer, 1);
}

#[test]
fn set_zoom_cam_action_sets_then_clears() {
    use crate::settings::model::CamZoomAction;
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 500,
        },
    );
    let id = doc.zooms[0].id.clone();
    assert_eq!(
        doc.zooms[0].cam_action, None,
        "new zooms inherit the global default"
    );

    apply(
        &mut doc,
        EditOp::SetZoomCamAction {
            id: id.clone(),
            action: Some(CamZoomAction::Stay),
        },
    );
    assert_eq!(doc.zooms[0].cam_action, Some(CamZoomAction::Stay));

    apply(&mut doc, EditOp::SetZoomCamAction { id, action: None });
    assert_eq!(
        doc.zooms[0].cam_action, None,
        "must be able to clear back to inherit"
    );
}

#[test]
fn add_zoom_appends_and_yields_distinct_ids() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 500,
            dur_ms: 1000,
        },
    );
    let z = &doc.zooms[0];
    assert_eq!(
        (z.start_ms, z.end_ms, z.scale, z.easing.as_str()),
        (500, 1500, 2.0, "smooth")
    );
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 200,
            dur_ms: 100,
        },
    );
    assert_ne!(doc.zooms[0].id, doc.zooms[1].id);
    assert!(doc.zooms[0].id.starts_with('z'));
}

#[test]
fn remove_zoom_drops_by_id() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 100,
        },
    );
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 200,
            dur_ms: 100,
        },
    );
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::RemoveZoom { id });
    assert_eq!(doc.zooms.len(), 1);
}

#[test]
fn add_zoom_full_uses_given_scale() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoomFull {
            at_ms: 100,
            dur_ms: 500,
            scale: 3.0,
        },
    );
    let z = &doc.zooms[0];
    assert_eq!((z.start_ms, z.end_ms, z.scale), (100, 600, 3.0));
    assert!(z.id.starts_with('z'));
}

#[path = "api_camera_tests.rs"]
mod camera_tests;
#[path = "api_clamp_tests.rs"]
mod clamp_tests;
#[path = "api_docops_tests.rs"]
mod docops_tests;
#[path = "api_layout_tests.rs"]
mod layout_tests;
#[path = "api_zoom_update_tests.rs"]
mod zoom_update_tests;
