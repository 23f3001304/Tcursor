use crate::edit::model::EditDoc;
use crate::edit::text::{TextAnchor, TextAnim, TextItem, TextKind, TextSize, TEXT_SIZE_FRACS};

#[test]
fn a_doc_written_before_texts_existed_loads_with_an_empty_list() {
    let json = r#"{"version":2,"trim":{"in_ms":0,"out_ms":0},"clip_ms":1000,"cuts":[],"zooms":[],
        "speed":[],"layout":[],"effects":[],"camera_moves":[],"aspect":"source",
        "settings":{},"captions":[]}"#;
    let doc: EditDoc = serde_json::from_str(json).unwrap();
    assert!(doc.texts.is_empty() && doc.clips.is_empty());
    assert_eq!(doc.version, 2);
}

#[test]
fn a_text_with_only_the_required_fields_takes_the_spec_defaults() {
    let t: TextItem =
        serde_json::from_str(r#"{"id":"t0","start_ms":100,"end_ms":2100,"text":"Hi"}"#).unwrap();
    assert_eq!(t.kind, TextKind::Title);
    assert_eq!(t.sub, None);
    assert_eq!(t.style, "clean");
    assert_eq!(t.pos, TextAnchor::BottomCenter);
    assert_eq!(t.offset, [0.0, 0.0]);
    assert_eq!(t.size, TextSize::M);
    assert_eq!((t.anim_in, t.anim_out), (TextAnim::Fade, TextAnim::Fade));
    assert_eq!((t.in_ms, t.out_ms), (420, 420));
    assert_eq!(t.easing, "smooth");
}

#[test]
fn the_enums_are_snake_and_lower_case_on_the_wire() {
    assert_eq!(
        serde_json::to_string(&TextKind::LowerThird).unwrap(),
        "\"lower_third\""
    );
    assert_eq!(
        serde_json::to_string(&TextAnchor::TopLeft).unwrap(),
        "\"top_left\""
    );
    assert_eq!(serde_json::to_string(&TextSize::Xl).unwrap(), "\"xl\"");
    assert_eq!(
        serde_json::to_string(&TextAnim::Typewriter).unwrap(),
        "\"typewriter\""
    );
}

#[test]
fn a_full_text_round_trips_through_a_doc_and_absent_sub_is_not_written() {
    let t = TextItem {
        id: "t0".into(),
        start_ms: 500,
        end_ms: 3500,
        kind: TextKind::LowerThird,
        text: "Name".into(),
        sub: Some("Role".into()),
        style: "bar".into(),
        pos: TextAnchor::BottomLeft,
        offset: [0.04, -0.06],
        size: TextSize::L,
        anim_in: TextAnim::Slide,
        anim_out: TextAnim::Pop,
        in_ms: 300,
        out_ms: 500,
        easing: "spring(300.000,10.000,1.000)".into(),
    };
    let mut doc = EditDoc::default();
    doc.texts = vec![t.clone()];
    let back: EditDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.texts, vec![t.clone()]);
    let title = TextItem { sub: None, ..t };
    assert!(!serde_json::to_string(&title).unwrap().contains("\"sub\""));
}

#[test]
fn the_size_rungs_are_the_spec_fractions_of_output_height() {
    assert_eq!(TEXT_SIZE_FRACS, [0.030, 0.042, 0.058, 0.082, 0.115]);
    assert_eq!(TextSize::M.frac(), 0.058);
}
