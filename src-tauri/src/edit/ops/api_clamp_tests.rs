// M5 inverted-region clamp tests (UpdateZoom/UpdateLayoutSeg), split out of api_tests.rs so
// that file stays under the size limit. Mirrors api_camera_tests.rs's split.
use super::*;

/// M5: dragging a zoom's start handle past its current end must not persist an inverted region
/// (`start > end`) - the untouched end handle is pulled up to meet it instead.
#[test]
fn update_zoom_start_past_end_pulls_end_to_match() {
    let mut doc = empty(); doc.trim.out_ms = 10_000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 1000, dur_ms: 1000 }); // [1000, 2000)
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id, start_ms: Some(8000), end_ms: None, scale: None,
        target: None, easing: None, zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms), (8000, 8000));
}

#[test]
fn update_zoom_end_before_start_pulls_start_to_match() {
    let mut doc = empty(); doc.trim.out_ms = 10_000;
    apply(&mut doc, EditOp::AddZoom { at_ms: 5000, dur_ms: 1000 }); // [5000, 6000)
    let id = doc.zooms[0].id.clone();
    apply(&mut doc, EditOp::UpdateZoom { id, start_ms: None, end_ms: Some(1000), scale: None,
        target: None, easing: None, zoom_in_ms: None, zoom_out_ms: None, layer: None });
    assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms), (1000, 1000));
}

#[test]
fn update_layout_seg_start_past_end_pulls_end_to_match() {
    let mut doc = empty(); doc.trim.out_ms = 10_000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 1000, dur_ms: 1000, layout: "camera".into(), transition_out_ms: None, easing_out: None });
    let id = doc.layout[doc.layout.len() - 1].id.clone();
    apply(&mut doc, EditOp::UpdateLayoutSeg { id: id.clone(), start_ms: Some(9000), end_ms: None,
        layout: None, transition_ms: None, easing: None, transition_out_ms: None, easing_out: None });
    let s = doc.layout.iter().find(|s| s.id == id).unwrap();
    assert_eq!((s.start_ms, s.end_ms), (9000, 9000));
}
