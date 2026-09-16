use crate::edit::clip::Clip;
use crate::edit::model::EditDoc;

#[test]
fn a_clip_without_a_transition_parses_as_a_hard_cut() {
    let c: Clip =
        serde_json::from_str(r#"{"id":"cl0","src_in_ms":500,"src_out_ms":4000}"#).unwrap();
    assert_eq!(c.transition_in_ms, 0);
}

#[test]
fn clips_round_trip_in_output_order() {
    let mut doc = EditDoc::default();
    doc.clips = vec![
        Clip {
            id: "cl1".into(),
            src_in_ms: 6000,
            src_out_ms: 9000,
            transition_in_ms: 0,
        },
        Clip {
            id: "cl0".into(),
            src_in_ms: 500,
            src_out_ms: 4000,
            transition_in_ms: 500,
        },
    ];
    let back: EditDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.clips, doc.clips);
}
