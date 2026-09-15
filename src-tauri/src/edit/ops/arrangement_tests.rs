use super::*;
use crate::edit::ops::api::apply;

fn pose(cx: f32, cy: f32, size: f32) -> PanelPose {
    PanelPose { cx, cy, size }
}

fn doc_with_seg() -> EditDoc {
    let mut doc = EditDoc::default();
    doc.clip_ms = 10_000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 4000,
            layout: "presenter".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    assert_eq!(
        doc.layout[0].arrangement, None,
        "a fresh segment starts on its preset"
    );
    doc
}

fn arr(doc: &EditDoc) -> Arrangement {
    doc.layout[0].arrangement.unwrap()
}

fn set(doc: &mut EditDoc, screen: Option<Option<PanelPose>>, cam: Option<Option<PanelPose>>) {
    apply(
        doc,
        EditOp::SetArrangement {
            id: "l0".into(),
            screen,
            cam,
        },
    );
}

#[test]
fn setting_both_panels_creates_the_arrangement() {
    let mut doc = doc_with_seg();
    set(
        &mut doc,
        Some(Some(pose(0.3, 0.5, 0.6))),
        Some(Some(pose(0.8, 0.7, 0.25))),
    );
    assert_eq!(
        arr(&doc),
        Arrangement {
            screen: Some(pose(0.3, 0.5, 0.6)),
            cam: Some(pose(0.8, 0.7, 0.25))
        }
    );
    assert_eq!(doc.layout[0].layout, "presenter");
}

#[test]
fn a_set_leaves_untouched_panels_alone_replaces_posed_ones_and_hides_nulled_ones() {
    let mut doc = doc_with_seg();
    set(
        &mut doc,
        Some(Some(pose(0.3, 0.5, 0.6))),
        Some(Some(pose(0.8, 0.7, 0.25))),
    );

    set(&mut doc, None, Some(Some(pose(0.5, 0.5, 0.4))));
    assert_eq!(
        arr(&doc).screen,
        Some(pose(0.3, 0.5, 0.6)),
        "untouched screen must not move"
    );
    assert_eq!(arr(&doc).cam, Some(pose(0.5, 0.5, 0.4)));

    set(&mut doc, None, Some(None));
    assert_eq!(arr(&doc).cam, None);
    assert_eq!(
        arr(&doc).screen,
        Some(pose(0.3, 0.5, 0.6)),
        "hiding one panel leaves the other"
    );

    set(&mut doc, None, None);
    assert_eq!(
        arr(&doc),
        Arrangement {
            screen: Some(pose(0.3, 0.5, 0.6)),
            cam: None
        }
    );
}

#[test]
fn a_first_set_on_a_bare_segment_leaves_the_untouched_panel_hidden() {
    let mut doc = doc_with_seg();
    set(&mut doc, None, Some(Some(pose(0.5, 0.5, 0.9))));
    assert_eq!(
        arr(&doc),
        Arrangement {
            screen: None,
            cam: Some(pose(0.5, 0.5, 0.9))
        }
    );
}

#[test]
fn hiding_the_last_visible_panel_is_rejected_and_changes_nothing() {
    let mut doc = doc_with_seg();
    set(&mut doc, Some(Some(pose(0.3, 0.5, 0.6))), Some(None));
    let before = doc.clone();
    set(&mut doc, Some(None), None);
    assert_eq!(
        doc, before,
        "the op must be a no-op, not an empty arrangement"
    );
    set(&mut doc, Some(None), Some(None));
    assert_eq!(doc, before);
    let mut bare = doc_with_seg();
    let bare_before = bare.clone();
    set(&mut bare, Some(None), Some(None));
    assert_eq!(bare, bare_before);
    assert_eq!(bare.layout[0].arrangement, None, "still on its preset");
}

#[test]
fn poses_are_clamped_on_the_way_in() {
    let mut doc = doc_with_seg();
    set(
        &mut doc,
        Some(Some(pose(-0.4, 1.9, 4.0))),
        Some(Some(pose(0.5, 0.5, 0.001))),
    );
    assert_eq!(
        arr(&doc).screen,
        Some(pose(0.0, 1.0, 1.5)),
        "cx/cy clamp to 0..1, size to 1.5"
    );
    assert_eq!(
        arr(&doc).cam,
        Some(pose(0.5, 0.5, 0.05)),
        "size clamps up to the 0.05 floor"
    );
}

#[test]
fn clear_arrangement_restores_the_preset() {
    let mut doc = doc_with_seg();
    set(
        &mut doc,
        Some(Some(pose(0.3, 0.5, 0.6))),
        Some(Some(pose(0.8, 0.7, 0.25))),
    );
    apply(&mut doc, EditOp::ClearArrangement { id: "l0".into() });
    assert_eq!(doc.layout[0].arrangement, None);
    apply(&mut doc, EditOp::ClearArrangement { id: "l0".into() });
    assert_eq!(doc.layout[0].arrangement, None);
}

#[test]
fn an_unknown_segment_id_is_a_no_op() {
    let mut doc = doc_with_seg();
    let before = doc.clone();
    set(&mut doc, Some(Some(pose(0.5, 0.5, 0.5))), None);
    apply(
        &mut doc,
        EditOp::SetArrangement {
            id: "nope".into(),
            screen: Some(Some(pose(0.1, 0.1, 0.1))),
            cam: None,
        },
    );
    apply(&mut doc, EditOp::ClearArrangement { id: "nope".into() });
    assert_eq!(
        doc.layout[0].arrangement,
        Some(Arrangement {
            screen: Some(pose(0.5, 0.5, 0.5)),
            cam: None
        })
    );
    assert_eq!(doc.layout.len(), before.layout.len());
}

#[test]
fn the_wire_form_distinguishes_an_absent_key_from_an_explicit_null() {
    let absent: EditOp = serde_json::from_str(r#"{"op":"set_arrangement","id":"l0"}"#).unwrap();
    assert_eq!(
        absent,
        EditOp::SetArrangement {
            id: "l0".into(),
            screen: None,
            cam: None
        }
    );

    let nulled: EditOp =
        serde_json::from_str(r#"{"op":"set_arrangement","id":"l0","cam":null}"#).unwrap();
    assert_eq!(
        nulled,
        EditOp::SetArrangement {
            id: "l0".into(),
            screen: None,
            cam: Some(None)
        }
    );

    let posed: EditOp = serde_json::from_str(
        r#"{"op":"set_arrangement","id":"l0","screen":{"cx":0.5,"cy":0.5,"size":0.8}}"#,
    )
    .unwrap();
    assert_eq!(
        posed,
        EditOp::SetArrangement {
            id: "l0".into(),
            screen: Some(Some(pose(0.5, 0.5, 0.8))),
            cam: None
        }
    );

    for op in [absent, nulled, posed] {
        assert_eq!(
            serde_json::from_str::<EditOp>(&serde_json::to_string(&op).unwrap()).unwrap(),
            op
        );
    }
    let clear: EditOp = serde_json::from_str(r#"{"op":"clear_arrangement","id":"l0"}"#).unwrap();
    assert_eq!(clear, EditOp::ClearArrangement { id: "l0".into() });
}

#[test]
fn an_untouched_panel_is_omitted_from_the_serialized_op() {
    let op = EditOp::SetArrangement {
        id: "l0".into(),
        screen: Some(Some(pose(0.5, 0.5, 0.8))),
        cam: None,
    };
    let json = serde_json::to_string(&op).unwrap();
    assert!(
        !json.contains("cam"),
        "untouched panel must not be written: {}",
        json
    );
    let hide = EditOp::SetArrangement {
        id: "l0".into(),
        screen: None,
        cam: Some(None),
    };
    assert!(serde_json::to_string(&hide)
        .unwrap()
        .contains(r#""cam":null"#));
}
