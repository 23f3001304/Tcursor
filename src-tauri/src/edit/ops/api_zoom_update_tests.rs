use super::*;

#[test]
fn update_zoom_clamps_end_to_trim_duration() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 500,
        },
    );
    let id = doc.zooms[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id,
            start_ms: None,
            end_ms: Some(5000),
            scale: None,
            target: None,
            easing: None,
            zoom_in_ms: None,
            zoom_out_ms: None,
            layer: None,
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(doc.zooms[0].end_ms, 1000);
}

#[test]
fn update_zoom_sets_layer_and_validates_easing() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 500,
        },
    );
    let id = doc.zooms[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id: id.clone(),
            start_ms: None,
            end_ms: None,
            scale: None,
            target: None,
            easing: Some("spring".into()),
            zoom_in_ms: None,
            zoom_out_ms: None,
            layer: Some(2),
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(
        (doc.zooms[0].layer, doc.zooms[0].easing.as_str()),
        (2, "spring")
    );
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id,
            start_ms: None,
            end_ms: None,
            scale: None,
            target: None,
            easing: Some("bogus".into()),
            zoom_in_ms: None,
            zoom_out_ms: None,
            layer: None,
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(doc.zooms[0].easing, "smooth");
}

#[test]
fn update_zoom_changes_fields_and_ignores_unknown() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 500,
        },
    );
    let id = doc.zooms[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id: id.clone(),
            start_ms: Some(100),
            end_ms: None,
            scale: None,
            target: None,
            easing: None,
            zoom_in_ms: None,
            zoom_out_ms: None,
            layer: None,
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(
        (
            doc.zooms[0].start_ms,
            doc.zooms[0].end_ms,
            doc.zooms[0].scale
        ),
        (100, 500, 2.0)
    );
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id: "z999".into(),
            start_ms: Some(9999),
            end_ms: None,
            scale: None,
            target: None,
            easing: None,
            zoom_in_ms: None,
            zoom_out_ms: None,
            layer: None,
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(doc.zooms[0].start_ms, 100);
}

#[test]
fn update_zoom_sets_durations() {
    let mut doc = empty();
    apply(
        &mut doc,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    let id = doc.zooms[0].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateZoom {
            id,
            start_ms: None,
            end_ms: None,
            scale: None,
            target: None,
            easing: None,
            zoom_in_ms: Some(120),
            zoom_out_ms: Some(640),
            layer: None,
            smart_typing: None,
            easing_out: None,
        },
    );
    assert_eq!(
        (doc.zooms[0].zoom_in_ms, doc.zooms[0].zoom_out_ms),
        (120, 640)
    );
}
