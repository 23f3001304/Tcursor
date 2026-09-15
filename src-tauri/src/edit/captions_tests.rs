use crate::edit::captions::{Caption, CaptionWord};
use crate::edit::model::EditDoc;

#[test]
fn a_doc_written_before_captions_existed_loads_with_an_empty_track() {
    let json = r#"{"version":2,"trim":{"in_ms":0,"out_ms":0},"clip_ms":1000,"cuts":[],"zooms":[],
        "speed":[],"layout":[],"effects":[],"camera_moves":[],"aspect":"source",
        "settings":{}}"#;
    let doc: EditDoc = serde_json::from_str(json).expect("old doc must still parse");
    assert!(doc.captions.is_empty());
    assert_eq!(doc.version, 2, "adding a field is not a schema bump");
}

#[test]
fn a_caption_round_trips_through_json_with_its_words() {
    let c = Caption {
        id: "c0".into(),
        start_ms: 1000,
        end_ms: 3000,
        text: "hello world".into(),
        words: vec![
            CaptionWord {
                start_ms: 1000,
                end_ms: 1500,
                text: "hello".into(),
            },
            CaptionWord {
                start_ms: 1500,
                end_ms: 3000,
                text: "world".into(),
            },
        ],
    };
    let mut doc = EditDoc::default();
    doc.captions = vec![c.clone()];
    let back: EditDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.captions, vec![c]);
}

#[test]
fn a_caption_without_words_still_parses() {
    let c: Caption =
        serde_json::from_str(r#"{"id":"c0","start_ms":0,"end_ms":900,"text":"typed by hand"}"#)
            .unwrap();
    assert!(c.words.is_empty());
}
