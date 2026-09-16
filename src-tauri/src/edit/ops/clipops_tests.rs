use super::*;
use crate::edit::model::{Cut, EditDoc, Speed};
use crate::edit::ops::api::EditOp;

fn doc() -> EditDoc {
    let mut d = EditDoc::default();
    d.clip_ms = 10_000;
    d.trim.in_ms = 500;
    d.trim.out_ms = 9000;
    d
}

fn ranges(d: &EditDoc) -> Vec<(u32, u32, u32)> {
    d.clips
        .iter()
        .map(|c| (c.src_in_ms, c.src_out_ms, c.transition_in_ms))
        .collect()
}

fn ids(d: &EditDoc) -> Vec<String> {
    d.clips.iter().map(|c| c.id.clone()).collect()
}

fn upd(id: &str, fields: &str) -> EditOp {
    serde_json::from_str(&format!(r#"{{"op":"update_clip","id":"{id}"{fields}}}"#)).unwrap()
}

#[test]
fn the_first_split_materialises_two_clips_over_the_trim() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    assert_eq!(ranges(&d), vec![(500, 4000, 0), (4000, 9000, 0)]);
    assert_eq!(ids(&d), ["cl0", "cl1"]);
}

#[test]
fn a_split_at_an_edge_outside_every_clip_or_on_an_unresolvable_doc_is_a_noop() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 500 });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 9000 });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 9500 });
    assert!(d.clips.is_empty());
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    assert_eq!(d.clips.len(), 2);
    let mut bare = EditDoc::default();
    apply_clip(&mut bare, EditOp::SplitAt { at_ms: 100 });
    assert!(bare.clips.is_empty());
}

#[test]
fn a_second_split_divides_the_clip_containing_it_and_keeps_the_order() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 2000 });
    assert_eq!(
        ranges(&d),
        vec![(500, 2000, 0), (2000, 4000, 0), (4000, 9000, 0)]
    );
    assert_eq!(ids(&d), ["cl0", "cl2", "cl1"]);
}

#[test]
fn a_split_inherits_the_cuts_and_speed_spans_that_straddle_it() {
    let mut d = doc();
    d.cuts.push(Cut {
        id: "c0".into(),
        start_ms: 3500,
        end_ms: 4500,
    });
    d.speed.push(Speed {
        id: "s0".into(),
        start_ms: 3000,
        end_ms: 5000,
        factor: 2.0,
    });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    assert_eq!(d.cuts.len(), 1);
    assert_eq!(d.speed.len(), 1);
    let m = crate::export::remap::TimeMap::build(&d.trim, &d.cuts, &d.speed, &d.clips, 10_000);
    assert_eq!(m.clip_out_ms(0), 2500 + 250);
    assert_eq!(m.clip_out_ms(1), 250 + 4000);
}

#[test]
fn move_clip_reorders_and_clamps_the_index() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 2000 });
    apply_clip(
        &mut d,
        EditOp::MoveClip {
            id: "cl1".into(),
            to_index: 0,
        },
    );
    assert_eq!(ids(&d), ["cl1", "cl0", "cl2"]);
    apply_clip(
        &mut d,
        EditOp::MoveClip {
            id: "cl1".into(),
            to_index: 99,
        },
    );
    assert_eq!(ids(&d), ["cl0", "cl2", "cl1"]);
    apply_clip(
        &mut d,
        EditOp::MoveClip {
            id: "nope".into(),
            to_index: 0,
        },
    );
    assert_eq!(ids(&d), ["cl0", "cl2", "cl1"]);
}

#[test]
fn update_clip_clamps_and_orders_the_source_range_and_drops_a_zero_length_clip() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    apply_clip(&mut d, upd("cl1", r#","src_out_ms":50000"#));
    assert_eq!(ranges(&d)[1], (4000, 10_000, 0));
    apply_clip(&mut d, upd("cl1", r#","src_in_ms":4500,"src_out_ms":4200"#));
    assert_eq!(ranges(&d)[1], (4200, 4500, 0));
    apply_clip(&mut d, upd("cl1", r#","src_in_ms":4500"#));
    assert_eq!(ids(&d), ["cl0"], "a clip squeezed to nothing is dropped");
}

#[test]
fn a_transition_is_clamped_to_two_seconds_and_half_the_shorter_neighbour() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 8000 });
    apply_clip(&mut d, upd("cl1", r#","transition_in_ms":5000"#));
    assert_eq!(ranges(&d)[1].2, 500, "half of the 1000 ms second clip");
    apply_clip(&mut d, upd("cl1", r#","src_out_ms":9000"#));
    apply_clip(&mut d, upd("cl1", r#","src_in_ms":2000"#));
    apply_clip(&mut d, upd("cl1", r#","transition_in_ms":5000"#));
    assert_eq!(ranges(&d)[1].2, MAX_TRANSITION_MS);
    d.speed.push(Speed {
        id: "s0".into(),
        start_ms: 2000,
        end_ms: 9000,
        factor: 8.0,
    });
    apply_clip(&mut d, upd("cl1", r#","transition_in_ms":5000"#));
    assert_eq!(
        ranges(&d)[1].2,
        437,
        "half of the 875 ms OUTPUT length at 8x"
    );
    apply_clip(&mut d, upd("cl0", r#","transition_in_ms":300"#));
    assert_eq!(
        ranges(&d)[0].2,
        300,
        "stored on the first clip, ignored by the renderer"
    );
}

#[test]
fn remove_clip_never_removes_the_last_one_and_never_normalises_back_to_empty() {
    let mut d = doc();
    apply_clip(&mut d, EditOp::SplitAt { at_ms: 4000 });
    apply_clip(&mut d, EditOp::RemoveClip { id: "cl0".into() });
    assert_eq!(ranges(&d), vec![(4000, 9000, 0)]);
    apply_clip(&mut d, EditOp::RemoveClip { id: "cl1".into() });
    assert_eq!(ranges(&d), vec![(4000, 9000, 0)]);
}

#[test]
fn the_four_ops_parse_from_their_wire_shapes() {
    let ops: Vec<EditOp> = [
        r#"{"op":"split_at","at_ms":4000}"#,
        r#"{"op":"move_clip","id":"cl1","to_index":0}"#,
        r#"{"op":"update_clip","id":"cl1","transition_in_ms":250}"#,
        r#"{"op":"remove_clip","id":"cl1"}"#,
    ]
    .iter()
    .map(|s| serde_json::from_str(s).unwrap())
    .collect();
    assert!(matches!(ops[0], EditOp::SplitAt { at_ms: 4000 }));
    assert!(matches!(ops[1], EditOp::MoveClip { to_index: 0, .. }));
    assert!(matches!(
        ops[2],
        EditOp::UpdateClip {
            src_in_ms: None,
            transition_in_ms: Some(250),
            ..
        }
    ));
    assert!(matches!(ops[3], EditOp::RemoveClip { .. }));
}
