use crate::edit::model::EditDoc;
use crate::edit::ops::api::{apply, EditOp};

fn doc() -> EditDoc { let mut d = EditDoc::default(); d.clip_ms = 10_000; d }

#[test]
fn add_cuts_is_one_op_that_lands_sorted_merged_and_with_ids() {
    let mut d = doc();
    apply(&mut d, EditOp::AddCuts { spans: vec![(5000, 6000), (1000, 2000), (1900, 2500)] });
    let got: Vec<(String, u32, u32)> = d.cuts.iter().map(|c| (c.id.clone(), c.start_ms, c.end_ms)).collect();
    assert_eq!(got, vec![("c0".into(), 1000, 2500), ("c1".into(), 5000, 6000)]);
}

#[test]
fn add_cut_merges_into_an_existing_cut_and_keeps_the_earlier_id() {
    let mut d = doc();
    apply(&mut d, EditOp::AddCut { start_ms: 1000, end_ms: 2000 });
    apply(&mut d, EditOp::AddCut { start_ms: 2000, end_ms: 3000 });
    assert_eq!(d.cuts.len(), 1);
    assert_eq!((d.cuts[0].id.as_str(), d.cuts[0].start_ms, d.cuts[0].end_ms), ("c0", 1000, 3000));
}

#[test]
fn update_and_remove_cut_by_id_and_a_zero_length_result_is_dropped() {
    let mut d = doc();
    apply(&mut d, EditOp::AddCut { start_ms: 1000, end_ms: 2000 });
    apply(&mut d, EditOp::UpdateCut { id: "c0".into(), start_ms: Some(1500), end_ms: None });
    assert_eq!(d.cuts[0].start_ms, 1500);
    apply(&mut d, EditOp::UpdateCut { id: "c0".into(), start_ms: Some(2000), end_ms: Some(2000) });
    assert!(d.cuts.is_empty(), "a zero-length cut is dropped");
    apply(&mut d, EditOp::AddCut { start_ms: 1000, end_ms: 2000 });
    apply(&mut d, EditOp::RemoveCut { id: "c0".into() }); // ids restart from the highest live one, so the re-added cut is c0 again
    assert!(d.cuts.is_empty());
}

#[test]
fn cuts_are_clamped_into_the_clip() {
    let mut d = doc();
    apply(&mut d, EditOp::AddCut { start_ms: 9000, end_ms: 20_000 });
    apply(&mut d, EditOp::AddCut { start_ms: 12_000, end_ms: 13_000 });
    let got: Vec<(u32, u32)> = d.cuts.iter().map(|c| (c.start_ms, c.end_ms)).collect();
    assert_eq!(got, vec![(9000, 10_000)], "a cut past the clip end is clamped; one entirely past it is dropped");
}

#[test]
fn speed_spans_never_overlap_and_the_factor_is_clamped() {
    let mut d = doc();
    apply(&mut d, EditOp::SetSpeed { start_ms: 1000, end_ms: 3000, factor: 40.0 });
    apply(&mut d, EditOp::SetSpeed { start_ms: 2000, end_ms: 4000, factor: 0.1 });
    let got: Vec<(u32, u32, f32)> = d.speed.iter().map(|s| (s.start_ms, s.end_ms, s.factor)).collect();
    assert_eq!(got, vec![(1000, 3000, 8.0), (3000, 4000, 0.25)]);
    apply(&mut d, EditOp::UpdateSpeed { id: "s0".into(), start_ms: None, end_ms: Some(3500), factor: Some(2.0) });
    let got: Vec<(u32, u32, f32)> = d.speed.iter().map(|s| (s.start_ms, s.end_ms, s.factor)).collect();
    assert_eq!(got, vec![(1000, 3500, 2.0), (3500, 4000, 0.25)], "the updated span pushes its successor's start");
    apply(&mut d, EditOp::RemoveSpeed { id: "s1".into() });
    assert_eq!(d.speed.len(), 1);
}

#[test]
fn a_doc_saved_before_cut_ids_loads_with_ids_assigned() {
    let json = r#"{"version":2,"trim":{"in_ms":0,"out_ms":0},"cuts":[{"start_ms":10,"end_ms":20},{"id":"c7","start_ms":30,"end_ms":40},{"start_ms":50,"end_ms":60}],"zooms":[],"speed":[],"layout":[]}"#;
    let mut d: EditDoc = serde_json::from_str(json).unwrap();
    d.assign_missing_ids();
    let ids: Vec<&str> = d.cuts.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["c8", "c7", "c9"]);
}
