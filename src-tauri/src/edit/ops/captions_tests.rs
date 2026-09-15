use crate::edit::captions::{Caption, CaptionWord};
use crate::edit::model::EditDoc;
use crate::edit::ops::api::{apply, EditOp};
use crate::edit::ops::captions::split_text;

fn word(start_ms: u32, end_ms: u32, text: &str) -> CaptionWord {
    CaptionWord {
        start_ms,
        end_ms,
        text: text.into(),
    }
}

fn doc_with_two_captions() -> EditDoc {
    let mut d = EditDoc {
        clip_ms: 20_000,
        ..Default::default()
    };
    d.captions = vec![
        Caption {
            id: "c0".into(),
            start_ms: 1000,
            end_ms: 3000,
            text: "hello there world".into(),
            words: vec![
                word(1000, 1400, "hello"),
                word(1400, 2000, "there"),
                word(2000, 3000, "world"),
            ],
        },
        Caption {
            id: "c1".into(),
            start_ms: 3200,
            end_ms: 5000,
            text: "and again".into(),
            words: vec![word(3200, 4000, "and"), word(4000, 5000, "again")],
        },
    ];
    d
}

#[test]
fn update_retimes_a_caption_and_keeps_start_before_end() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: Some(1500),
            end_ms: None,
            text: None,
        },
    );
    assert_eq!((d.captions[0].start_ms, d.captions[0].end_ms), (1500, 3000));
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: Some(4000),
            end_ms: None,
            text: None,
        },
    );
    assert!(d.captions[0].end_ms > d.captions[0].start_ms);
}

#[test]
fn update_clamps_to_the_clip_and_ignores_an_unknown_id() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: None,
            end_ms: Some(99_000),
            text: None,
        },
    );
    assert_eq!(d.captions[0].end_ms, 20_000);
    let before = d.captions.clone();
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "nope".into(),
            start_ms: Some(0),
            end_ms: None,
            text: None,
        },
    );
    assert_eq!(d.captions, before);
}

#[test]
fn editing_the_text_keeps_the_word_timings_it_still_matches_and_drops_them_when_it_does_not() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: None,
            end_ms: None,
            text: Some("hello there world".into()),
        },
    );
    assert_eq!(
        d.captions[0].words.len(),
        3,
        "an unchanged text keeps its highlight timings"
    );
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: None,
            end_ms: None,
            text: Some("completely different copy".into()),
        },
    );
    assert!(
        d.captions[0].words.is_empty(),
        "retyped copy cannot keep stale word timings"
    );
    assert_eq!(d.captions[0].text, "completely different copy");
}

#[test]
fn remove_and_clear_do_what_they_say() {
    let mut d = doc_with_two_captions();
    apply(&mut d, EditOp::RemoveCaption { id: "c0".into() });
    assert_eq!(
        d.captions.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["c1"]
    );
    apply(&mut d, EditOp::ClearCaptions);
    assert!(d.captions.is_empty());
}

#[test]
fn set_captions_replaces_the_whole_track_and_leaves_it_sorted() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::SetCaptions {
            captions: vec![
                Caption {
                    id: "c1".into(),
                    start_ms: 800,
                    end_ms: 900,
                    text: "second".into(),
                    words: vec![],
                },
                Caption {
                    id: "c0".into(),
                    start_ms: 100,
                    end_ms: 200,
                    text: "first".into(),
                    words: vec![],
                },
            ],
        },
    );
    assert_eq!(
        d.captions
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
}

#[test]
fn merge_joins_a_caption_with_the_next_one_in_time() {
    let mut d = doc_with_two_captions();
    apply(&mut d, EditOp::MergeCaptions { id: "c0".into() });
    assert_eq!(d.captions.len(), 1);
    let c = &d.captions[0];
    assert_eq!(
        c.id, "c0",
        "the earlier caption's id survives so the selection does not jump"
    );
    assert_eq!((c.start_ms, c.end_ms), (1000, 5000));
    assert_eq!(c.text, "hello there world and again");
    assert_eq!(c.words.len(), 5);
}

#[test]
fn merging_the_last_caption_is_a_no_op() {
    let mut d = doc_with_two_captions();
    let before = d.captions.clone();
    apply(&mut d, EditOp::MergeCaptions { id: "c1".into() });
    assert_eq!(d.captions, before);
}

#[test]
fn split_cuts_at_the_playhead_and_hands_each_half_its_own_words() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::SplitCaption {
            id: "c0".into(),
            at_ms: 2000,
        },
    );
    assert_eq!(d.captions.len(), 3);
    let (a, b) = (&d.captions[0], &d.captions[1]);
    assert_eq!(a.id, "c0");
    assert_eq!((a.start_ms, a.end_ms), (1000, 2000));
    assert_eq!(a.text, "hello there");
    assert_eq!((b.start_ms, b.end_ms), (2000, 3000));
    assert_eq!(b.text, "world");
    assert_ne!(b.id, a.id);
    assert!(
        d.captions.iter().any(|c| c.id == "c1"),
        "the untouched caption is still there"
    );
}

#[test]
fn split_outside_the_caption_is_a_no_op() {
    let mut d = doc_with_two_captions();
    let before = d.captions.clone();
    apply(
        &mut d,
        EditOp::SplitCaption {
            id: "c0".into(),
            at_ms: 1000,
        },
    );
    apply(
        &mut d,
        EditOp::SplitCaption {
            id: "c0".into(),
            at_ms: 9000,
        },
    );
    assert_eq!(d.captions, before);
}

#[test]
fn splitting_a_caption_with_no_word_timings_cuts_its_text_at_a_space() {
    let mut d = EditDoc {
        clip_ms: 10_000,
        ..Default::default()
    };
    d.captions = vec![Caption {
        id: "c0".into(),
        start_ms: 0,
        end_ms: 1000,
        text: "typed by hand here".into(),
        words: vec![],
    }];
    apply(
        &mut d,
        EditOp::SplitCaption {
            id: "c0".into(),
            at_ms: 500,
        },
    );
    assert_eq!(d.captions.len(), 2);
    assert_eq!(d.captions[0].text, "typed by");
    assert_eq!(d.captions[1].text, "hand here");
}

#[test]
fn split_text_never_cuts_inside_a_word_and_never_returns_an_empty_half() {
    assert_eq!(
        split_text("typed by hand here", 0.5),
        ("typed by".into(), "hand here".into())
    );
    assert_eq!(split_text("one two", 0.01), ("one".into(), "two".into()));
    assert_eq!(split_text("one two", 0.99), ("one".into(), "two".into()));
    assert_eq!(split_text("single", 0.5), ("single".into(), String::new()));
}

#[test]
fn the_track_stays_sorted_by_start_after_every_op() {
    let mut d = doc_with_two_captions();
    apply(
        &mut d,
        EditOp::UpdateCaption {
            id: "c0".into(),
            start_ms: Some(9000),
            end_ms: Some(9500),
            text: None,
        },
    );
    let starts: Vec<u32> = d.captions.iter().map(|c| c.start_ms).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted);
}
