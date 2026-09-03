// Layout-segment TRANSITION-default tests, split out of api_tests.rs so that file stays under the
// size limit. Mirrors api_camera_tests.rs's split. The placement/clamping/preset-validation tests
// for the same ops still live in api_tests.rs; this file owns only the entry-vs-exit defaults,
// which are the pair the "layout segments fade in but hard-cut out" bug turned on.
use super::*;

#[test]
fn add_layout_seg_is_symmetric_by_default_entry_and_exit() {
    // The exit used to default to `0` (a hard cut) while the entry got 350ms, so every segment a
    // user created faded IN and then snapped OUT. A new segment is symmetric unless asked otherwise.
    let mut doc = empty(); doc.trim.out_ms = 5000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 0, dur_ms: 2000, layout: "camera".into(), transition_out_ms: None, easing_out: None });
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!(s.transition_out_ms, NEW_LAYOUT_TRANSITION_MS);
    assert_eq!(s.transition_ms, s.transition_out_ms, "entry and exit must match");
    assert_eq!(s.easing_out, s.easing);
}

#[test]
fn add_layout_seg_explicit_exit_still_wins_over_the_default() {
    // Including an explicit hard cut: the new default must seed, never overwrite.
    let mut doc = empty(); doc.trim.out_ms = 5000;
    apply(&mut doc, EditOp::AddLayoutSeg { at_ms: 2000, dur_ms: 1000, layout: "camera".into(),
        transition_out_ms: Some(0), easing_out: Some("linear".into()) });
    let s = &doc.layout[doc.layout.len() - 1];
    assert_eq!((s.transition_out_ms, s.easing_out.as_str()), (0, "linear"));
}

#[test]
fn add_layout_seg_default_does_not_leak_into_loading_an_old_doc() {
    // Back-compat pin, paired with the op default above: the CREATION default is 350, but the
    // SERDE default for a doc saved before exit transitions existed must stay a `0` hard cut, so
    // an existing project keeps rendering byte-identically. Two different defaults on purpose.
    let json = r#"{"id":"l0","start_ms":0,"end_ms":1000,"layout":"camera","transition_ms":350,"easing":"smooth"}"#;
    let s: crate::edit::model::LayoutSeg = serde_json::from_str(json).unwrap();
    assert_eq!(s.transition_out_ms, 0);
    assert_ne!(s.transition_out_ms, NEW_LAYOUT_TRANSITION_MS);
}
