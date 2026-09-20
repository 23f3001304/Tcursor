use super::*;

fn steps(plan: &[u64]) -> Vec<(u64, u64, u64)> {
    let mut c = PlanCursor::new(plan.to_vec());
    let mut v = Vec::new();
    while let Some(s) = c.next() {
        v.push((s.j, s.k, s.decodes_needed));
    }
    v
}

#[test]
fn a_plain_plan_decodes_one_frame_per_output_frame_after_the_warm_up() {
    assert_eq!(steps(&[3, 4, 5]), vec![(0, 3, 4), (1, 4, 1), (2, 5, 1)]);
}

#[test]
fn a_cut_or_a_fast_span_skips_frames() {
    assert_eq!(
        steps(&[0, 2, 4, 40]),
        vec![(0, 0, 1), (1, 2, 2), (2, 4, 2), (3, 40, 36)]
    );
}

#[test]
fn slow_motion_reuses_the_held_frame() {
    assert_eq!(
        steps(&[7, 7, 8, 8]),
        vec![(0, 7, 8), (1, 7, 0), (2, 8, 1), (3, 8, 0)]
    );
}

#[test]
fn an_empty_plan_yields_nothing() {
    assert!(steps(&[]).is_empty());
    assert!(PlanCursor::new(vec![]).is_empty());
    assert_eq!(PlanCursor::new(vec![1, 2]).len(), 2);
}

fn rebased(plan: &[u64], at: usize, base: u64) -> Vec<(u64, u64, u64)> {
    let mut c = PlanCursor::new(plan.to_vec());
    let mut v = Vec::new();
    while let Some(s) = c.next() {
        v.push((s.j, s.k, s.decodes_needed));
        if s.j as usize + 1 == at {
            c.rebase(base);
        }
    }
    v
}

#[test]
fn a_cursor_that_is_never_rebased_is_the_shipped_sequence() {
    assert_eq!(steps(&[3, 4, 5]), rebased(&[3, 4, 5], usize::MAX, 0));
    assert_eq!(
        steps(&[0, 2, 4, 40]),
        rebased(&[0, 2, 4, 40], usize::MAX, 0)
    );
    assert_eq!(steps(&[7, 7, 8, 8]), rebased(&[7, 7, 8, 8], usize::MAX, 0));
}

#[test]
fn a_rebase_counts_the_first_decode_from_the_seek_not_from_the_file() {
    assert_eq!(
        rebased(&[0, 1, 900, 901], 2, 900),
        vec![(0, 0, 1), (1, 1, 1), (2, 900, 1), (3, 901, 1)]
    );
}

#[test]
fn a_rebase_past_the_frame_it_wants_never_asks_for_a_negative_decode() {
    assert_eq!(rebased(&[0, 5], 1, 9), vec![(0, 0, 1), (1, 5, 1)]);
}

fn split(a: u32, b: u32, c: u32) -> crate::export::remap::TimeMap {
    use crate::edit::clip::Clip;
    let clip = |id: &str, src_in_ms, src_out_ms| Clip {
        id: id.into(),
        src_in_ms,
        src_out_ms,
        transition_in_ms: 0,
    };
    crate::export::remap::TimeMap::build(
        &crate::edit::model::Trim {
            in_ms: 500,
            out_ms: 9000,
        },
        &[],
        &[],
        &[clip("cl0", a, b), clip("cl1", b, c)],
        10_000,
    )
}

#[test]
fn a_reordered_join_respawns_and_names_the_outgoing_clip() {
    let m = crate::export::remap::clips_tests::clips_fixture();
    let (spans, plan) = (m.clip_spans(10), m.frame_plan(10));
    assert_eq!(
        join_at(&spans, &plan, 49),
        Some(ClipJoin {
            first_k: 5,
            prev_clip: 0,
            respawn: true
        }),
        "frame 5 is behind the 89 the outgoing clip ended on"
    );
    for j in [0, 48, 50] {
        assert_eq!(join_at(&spans, &plan, j), None, "index {j}");
    }
}

#[test]
fn a_forward_join_is_decoded_through_and_never_respawns() {
    let m = split(500, 4000, 9000);
    let (spans, plan) = (m.clip_spans(10), m.frame_plan(10));
    assert_eq!(
        (spans[0].plan_len, spans[1].plan_start, spans[1].first_k),
        (35, 35, 40)
    );
    assert_eq!(
        join_at(&spans, &plan, 35),
        Some(ClipJoin {
            first_k: 40,
            prev_clip: 0,
            respawn: false
        }),
        "frame 40 is ahead of the 39 the outgoing clip ended on"
    );
    for j in [0, 34, 36] {
        assert_eq!(join_at(&spans, &plan, j), None, "index {j}");
    }
}

#[test]
fn a_document_with_no_clips_has_one_span_and_no_join_anywhere() {
    let m = crate::export::remap::tests::fixture();
    let (spans, plan) = (m.clip_spans(10), m.frame_plan(10));
    assert_eq!(spans.len(), 1);
    assert!(
        (0..plan.len()).all(|j| join_at(&spans, &plan, j).is_none()),
        "no clips, no join"
    );
}
