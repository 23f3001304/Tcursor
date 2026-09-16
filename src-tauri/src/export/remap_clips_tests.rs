use super::tests::{fixture, fixture_parts};
use super::*;

pub(crate) fn clips_fixture() -> TimeMap {
    use crate::edit::clip::Clip;
    let base = fixture_parts();
    TimeMap::build(
        &base.0,
        &base.1,
        &base.2,
        &[
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
                transition_in_ms: 0,
            },
        ],
        10_000,
    )
}

#[test]
fn clips_in_output_order_concatenate_their_kept_pieces() {
    let m = clips_fixture();
    let got: Vec<(usize, u32, u32, f64, f64)> = m
        .segments()
        .iter()
        .map(|s| (s.clip, s.clip_start, s.clip_end, s.factor, s.out_start))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, 6000, 8000, 0.5, 0.0),
            (0, 8000, 9000, 1.0, 4000.0),
            (1, 500, 1000, 1.0, 5000.0),
            (1, 2000, 2500, 1.0, 5500.0),
            (1, 2500, 3500, 2.0, 6000.0),
            (1, 3500, 4000, 1.0, 6500.0),
        ]
    );
    assert_eq!(m.out_dur_ms(), 7000);
    assert!(!m.is_plain());
    assert_eq!(
        (m.clip_out_ms(0), m.clip_out_ms(1), m.clip_out_ms(2)),
        (5000, 2000, 0)
    );
}

#[test]
fn out_of_and_clip_of_are_the_clips_parity_table() {
    let m = clips_fixture();
    for (clip, out) in [
        (0, 5000),
        (500, 5000),
        (750, 5250),
        (1500, 5500),
        (2250, 5750),
        (3000, 6250),
        (3750, 6750),
        (4000, 0),
        (4250, 0),
        (6000, 0),
        (7000, 2000),
        (8000, 4000),
        (8500, 4500),
        (9000, 7000),
        (9999, 7000),
    ] {
        assert_eq!(m.out_of(clip), out, "out_of({clip})");
    }
    for (out, clip) in [
        (0, 6000),
        (1000, 6500),
        (4000, 8000),
        (4500, 8500),
        (5000, 500),
        (5250, 750),
        (5500, 2000),
        (6000, 2500),
        (6250, 3000),
        (6500, 3500),
        (6750, 3750),
        (7000, 4000),
    ] {
        assert_eq!(m.clip_of(out), clip, "clip_of({out})");
    }
}

#[test]
fn a_reordered_frame_plan_is_the_concatenation_and_is_not_monotone() {
    let plan = clips_fixture().frame_plan(10);
    let mut expect: Vec<u64> = (60..=79).flat_map(|k| [k, k]).take(39).collect();
    expect.extend(80..=89);
    expect.extend(5..=9);
    expect.extend(20..=24);
    expect.extend([25, 27, 29, 31, 33]);
    expect.extend(35..=40);
    assert_eq!(plan, expect);
    assert!(plan.windows(2).any(|w| w[0] > w[1]));
}

#[test]
fn crosses_boundary_is_true_only_across_a_non_contiguous_segment_join() {
    let m = fixture();
    for (a, b, want) in [
        (499, 500, true),
        (999, 1000, false),
        (1499, 1500, false),
        (1999, 2000, true),
        (3499, 3500, false),
        (7499, 7500, false),
        (0, 0, false),
    ] {
        assert_eq!(m.crosses_boundary(a, b), want, "base ({a},{b})");
    }
    let c = clips_fixture();
    for (a, b, want) in [
        (3999, 4000, false),
        (4999, 5000, true),
        (5499, 5500, true),
        (5999, 6000, false),
        (6499, 6500, false),
        (100, 101, false),
    ] {
        assert_eq!(c.crosses_boundary(a, b), want, "clips ({a},{b})");
    }
}

#[test]
fn plan_boundaries_are_the_plan_indices_that_open_a_non_contiguous_segment() {
    assert_eq!(fixture().plan_boundaries(10), vec![5, 20]);
    assert_eq!(clips_fixture().plan_boundaries(10), vec![49, 54]);
    let cut = |a: u32, b: u32| {
        TimeMap::build(
            &Trim::default(),
            &[Cut {
                id: "x".into(),
                start_ms: a,
                end_ms: b,
            }],
            &[],
            &[],
            10_000,
        )
    };
    let sub_frame = cut(1003, 1015);
    assert_eq!(sub_frame.plan_boundaries(30), vec![31]);
    assert_eq!(
        sub_frame.frame_plan(30),
        (0..=300).collect::<Vec<u64>>(),
        "a cut shorter than a frame removes no plan entry"
    );
    assert!(cut(0, 10_000).plan_boundaries(30).is_empty());
    let fractional = TimeMap::build(
        &Trim::default(),
        &[],
        &[Speed {
            id: "s".into(),
            start_ms: 0,
            end_ms: 1533,
            factor: 2.0,
        }],
        &[],
        10_000,
    );
    assert!(
        fractional.plan_boundaries(30).is_empty(),
        "contiguous at 1533, whose output time 766.5 the output clock rounds past a frame late"
    );
}

#[test]
fn a_segment_carries_the_index_of_its_clip_in_the_list_as_written() {
    use crate::edit::clip::Clip;
    let clip = |id: &str, a, b| Clip {
        id: id.into(),
        src_in_ms: a,
        src_out_ms: b,
        transition_in_ms: 0,
    };
    let base = fixture_parts();
    let m = TimeMap::build(
        &base.0,
        &base.1,
        &base.2,
        &[
            clip("cl1", 6000, 9000),
            clip("bad", 6000, 2000),
            clip("cl0", 500, 4000),
        ],
        10_000,
    );
    let got: Vec<(usize, u32, u32, f64, f64)> = m
        .segments()
        .iter()
        .map(|s| (s.clip, s.clip_start, s.clip_end, s.factor, s.out_start))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, 6000, 8000, 0.5, 0.0),
            (0, 8000, 9000, 1.0, 4000.0),
            (2, 500, 1000, 1.0, 5000.0),
            (2, 2000, 2500, 1.0, 5500.0),
            (2, 2500, 3500, 2.0, 6000.0),
            (2, 3500, 4000, 1.0, 6500.0),
        ],
        "the dropped middle clip leaves a gap at index 1, it does not renumber the rest"
    );
    assert_eq!(
        (m.clip_out_ms(0), m.clip_out_ms(1), m.clip_out_ms(2)),
        (5000, 0, 2000)
    );
}

#[test]
fn an_empty_clip_list_is_the_trim_and_one_clip_narrower_than_the_trim_is_not_plain() {
    use crate::edit::clip::Clip;
    let trim = Trim {
        in_ms: 500,
        out_ms: 9000,
    };
    let one = |a, b| {
        TimeMap::build(
            &trim,
            &[],
            &[],
            &[Clip {
                id: "cl0".into(),
                src_in_ms: a,
                src_out_ms: b,
                transition_in_ms: 0,
            }],
            10_000,
        )
    };
    assert_eq!(
        TimeMap::build(&trim, &[], &[], &[], 10_000).segments(),
        one(500, 9000).segments(),
        "an empty list is the resolved trim, clip index and all"
    );
    assert!(one(500, 9000).is_plain());
    assert!(!one(500, 8000).is_plain());
    assert_eq!(
        one(6000, 2000).segments().len(),
        0,
        "an inverted clip range is dropped"
    );
}
