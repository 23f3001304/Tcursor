use super::*;

#[test]
fn add_layout_seg_is_symmetric_by_default_entry_and_exit() {
    let mut doc = empty();
    doc.trim.out_ms = 5000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 2000,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!(s.transition_out_ms, NEW_LAYOUT_TRANSITION_MS);
    assert_eq!(
        s.transition_ms, s.transition_out_ms,
        "entry and exit must match"
    );
    assert_eq!(s.easing_out, s.easing);
}

#[test]
fn add_layout_seg_explicit_exit_still_wins_over_the_default() {
    let mut doc = empty();
    doc.trim.out_ms = 5000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 2000,
            dur_ms: 1000,
            layout: "camera".into(),
            transition_out_ms: Some(0),
            easing_out: Some("linear".into()),
        },
    );
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!((s.transition_out_ms, s.easing_out.as_str()), (0, "linear"));
}

#[test]
fn add_layout_seg_default_does_not_leak_into_loading_an_old_doc() {
    let json = r#"{"id":"l0","start_ms":0,"end_ms":1000,"layout":"camera","transition_ms":350,"easing":"smooth"}"#;
    let s: crate::edit::model::LayoutSeg = serde_json::from_str(json).unwrap();
    assert_eq!(s.transition_out_ms, 0);
    assert_ne!(s.transition_out_ms, NEW_LAYOUT_TRANSITION_MS);
}

#[test]
fn add_layout_seg_appends_clamped_with_default_feel() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 800,
            dur_ms: 2000,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!(
        (s.start_ms, s.end_ms, s.layout.as_str()),
        (800, 1000, "camera")
    );
    assert_eq!((s.transition_ms, s.easing.as_str()), (350, "smooth"));
    assert!(s.id.starts_with('l'));
}

#[test]
fn add_layout_seg_rejects_unknown_preset() {
    let mut doc = empty();
    doc.trim.out_ms = 5000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "bogus".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    assert_eq!(doc.layout[doc.layout.len() - 1].layout, "screen");
}

#[test]
fn update_layout_seg_patches_feel_and_preset() {
    let mut doc = empty();
    doc.trim.out_ms = 5000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "screen".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    let id = doc.layout[doc.layout.len() - 1].id.clone();
    apply(
        &mut doc,
        EditOp::UpdateLayoutSeg {
            id: id.clone(),
            start_ms: None,
            end_ms: None,
            layout: Some("presenter".into()),
            transition_ms: Some(120),
            easing: Some("spring".into()),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    let s = doc.layout.iter().find(|s| s.id == id).unwrap();
    assert_eq!(
        (s.layout.as_str(), s.transition_ms, s.easing.as_str()),
        ("presenter", 120, "spring")
    );
}

#[test]
fn remove_layout_seg_drops_by_id() {
    let mut doc = empty();
    doc.trim.out_ms = 5000;
    apply(
        &mut doc,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 500,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    let id = doc.layout[doc.layout.len() - 1].id.clone();
    let before = doc.layout.len();
    apply(&mut doc, EditOp::RemoveLayoutSeg { id });
    assert_eq!(doc.layout.len(), before - 1);
}
