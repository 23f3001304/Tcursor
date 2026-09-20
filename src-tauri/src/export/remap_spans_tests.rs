use super::ClipSpan;
use crate::edit::clip::Clip;
use crate::edit::model::Trim;
use crate::export::remap::TimeMap;

fn clip(id: &str, a: u32, b: u32) -> Clip {
    Clip {
        id: id.into(),
        src_in_ms: a,
        src_out_ms: b,
        transition_in_ms: 0,
    }
}

fn trim() -> Trim {
    Trim {
        in_ms: 500,
        out_ms: 9000,
    }
}

fn plain(clips: &[Clip]) -> TimeMap {
    TimeMap::build(&trim(), &[], &[], clips, 10_000)
}

#[test]
fn an_empty_clip_list_is_one_span_covering_the_whole_plan() {
    let m = crate::export::remap::tests::fixture();
    let plan = m.frame_plan(10);
    assert_eq!(plan.len(), 85);
    assert_eq!(
        m.clip_spans(10),
        vec![ClipSpan {
            clip: 0,
            plan_start: 0,
            plan_len: 85,
            first_k: 5,
        }]
    );
}

#[test]
fn a_split_in_order_reproduces_the_unsplit_plan_exactly() {
    let whole = plain(&[]).frame_plan(10);
    assert_eq!(whole, (5..=90).collect::<Vec<u64>>());
    let aligned = plain(&[clip("cl0", 500, 4000), clip("cl1", 4000, 9000)]);
    assert_eq!(aligned.frame_plan(10), whole, "a split on a frame boundary");
    let off = plain(&[clip("cl0", 500, 4050), clip("cl1", 4050, 9000)]);
    assert_eq!(off.frame_plan(10), whole, "a split between two frames");
    assert_eq!(
        aligned.clip_spans(10),
        vec![
            ClipSpan {
                clip: 0,
                plan_start: 0,
                plan_len: 35,
                first_k: 5
            },
            ClipSpan {
                clip: 1,
                plan_start: 35,
                plan_len: 51,
                first_k: 40
            },
        ]
    );
    assert_eq!(
        off.clip_spans(10),
        vec![
            ClipSpan {
                clip: 0,
                plan_start: 0,
                plan_len: 36,
                first_k: 5
            },
            ClipSpan {
                clip: 1,
                plan_start: 36,
                plan_len: 50,
                first_k: 41
            },
        ]
    );
}

#[test]
fn a_reorder_keeps_the_frame_count_and_moves_the_splice_frame() {
    let m = plain(&[clip("cl0", 4000, 9000), clip("cl1", 500, 4000)]);
    let plan = m.frame_plan(10);
    assert_eq!(plan.len(), 86, "the same count as the unsplit plan");
    assert_eq!(plan.first().copied(), Some(40));
    assert_eq!(
        plan.last().copied(),
        Some(40),
        "the last clip floors its end"
    );
    assert!(
        !plan.contains(&90),
        "the recording's final frame is not shown"
    );
    assert_eq!(
        m.clip_spans(10),
        vec![
            ClipSpan {
                clip: 0,
                plan_start: 0,
                plan_len: 50,
                first_k: 40
            },
            ClipSpan {
                clip: 1,
                plan_start: 50,
                plan_len: 36,
                first_k: 5
            },
        ]
    );
}

#[test]
fn the_clips_fixtures_spans_line_up_with_its_seventy_entry_plan() {
    let m = crate::export::remap::clips_tests::clips_fixture();
    assert_eq!(m.frame_plan(10).len(), 70);
    assert_eq!(
        m.clip_spans(10),
        vec![
            ClipSpan {
                clip: 0,
                plan_start: 0,
                plan_len: 49,
                first_k: 60
            },
            ClipSpan {
                clip: 1,
                plan_start: 49,
                plan_len: 21,
                first_k: 5
            },
        ]
    );
    assert_eq!(
        m.plan_boundaries(10),
        vec![49, 54],
        "a clip join AND a cut join; only the first needs a decoder"
    );
}

#[test]
fn a_clip_that_contributes_no_frames_gets_no_span() {
    let m = TimeMap::build(
        &trim(),
        &[],
        &[],
        &[clip("cl0", 2000, 5000), clip("cl1", 500, 501)],
        10_000,
    );
    let spans = m.clip_spans(60);
    assert_eq!(spans.len(), 2, "at 60 fps both clips still land a frame");
    let coarse = m.clip_spans(3);
    assert_eq!(
        coarse.len(),
        1,
        "at 3 fps the one-millisecond clip has none"
    );
    assert_eq!(coarse[0].clip, 0);
    assert_eq!(coarse[0].plan_start, 0);
}

#[test]
fn the_spans_partition_the_plan() {
    for m in [
        crate::export::remap::tests::fixture(),
        crate::export::remap::clips_tests::clips_fixture(),
        plain(&[clip("cl0", 4000, 9000), clip("cl1", 500, 4000)]),
    ] {
        for fps in [10u64, 24, 30, 60] {
            let plan = m.frame_plan(fps);
            let spans = m.clip_spans(fps);
            let mut at = 0usize;
            for s in &spans {
                assert_eq!(s.plan_start, at, "fps {fps}");
                assert_eq!(plan[s.plan_start], s.first_k, "fps {fps}");
                at += s.plan_len;
            }
            assert_eq!(at, plan.len(), "fps {fps}");
        }
    }
}
