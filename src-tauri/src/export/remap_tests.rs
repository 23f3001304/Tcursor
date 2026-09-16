use super::*;
use crate::edit::model::{Cut, Speed, Trim};

pub(crate) fn fixture_parts() -> (Trim, Vec<Cut>, Vec<Speed>) {
    (
        Trim {
            in_ms: 500,
            out_ms: 9000,
        },
        vec![
            Cut {
                id: "c0".into(),
                start_ms: 1000,
                end_ms: 2000,
            },
            Cut {
                id: "c1".into(),
                start_ms: 4000,
                end_ms: 4500,
            },
        ],
        vec![
            Speed {
                id: "s0".into(),
                start_ms: 2500,
                end_ms: 3500,
                factor: 2.0,
            },
            Speed {
                id: "s1".into(),
                start_ms: 6000,
                end_ms: 8000,
                factor: 0.5,
            },
        ],
    )
}

pub(crate) fn fixture() -> TimeMap {
    let p = fixture_parts();
    TimeMap::build(&p.0, &p.1, &p.2, &[], 10_000)
}

#[test]
fn segments_are_the_kept_pieces_with_their_factors_and_output_starts() {
    let m = fixture();
    let got: Vec<(u32, u32, f64, f64)> = m
        .segments()
        .iter()
        .map(|s| (s.clip_start, s.clip_end, s.factor, s.out_start))
        .collect();
    assert_eq!(
        got,
        vec![
            (500, 1000, 1.0, 0.0),
            (2000, 2500, 1.0, 500.0),
            (2500, 3500, 2.0, 1000.0),
            (3500, 4000, 1.0, 1500.0),
            (4500, 6000, 1.0, 2000.0),
            (6000, 8000, 0.5, 3500.0),
            (8000, 9000, 1.0, 7500.0)
        ]
    );
    assert_eq!(m.out_dur_ms(), 8500);
    assert!(!m.is_plain());
}

#[test]
fn out_of_is_the_parity_table() {
    let m = fixture();
    let table = [
        (0, 0),
        (500, 0),
        (1200, 500),
        (2000, 500),
        (3000, 1250),
        (3500, 1500),
        (4250, 2000),
        (5000, 2500),
        (7000, 5500),
        (9000, 8500),
        (9999, 8500),
    ];
    for (clip, out) in table {
        assert_eq!(m.out_of(clip), out, "out_of({clip})");
    }
}

#[test]
fn clip_of_is_the_parity_table_and_inverts_out_of_on_kept_ranges() {
    let m = fixture();
    let table = [
        (0, 500),
        (250, 750),
        (500, 2000),
        (1250, 3000),
        (1500, 3500),
        (2000, 4500),
        (5500, 7000),
        (8500, 9000),
        (9000, 9000),
    ];
    for (out, clip) in table {
        assert_eq!(m.clip_of(out), clip, "clip_of({out})");
    }
    for t in [500u32, 900, 2200, 3000, 3900, 5000, 7500, 8999] {
        assert_eq!(m.clip_of(m.out_of(t)), t, "round trip {t}");
    }
}

#[test]
fn frame_plan_at_10fps_keeps_the_trim_edges_and_removes_exactly_the_cut_frames() {
    let m = fixture();
    let plan = m.frame_plan(10);
    let mut expect: Vec<u64> = (5..=9).collect();
    expect.extend(20..=24);
    expect.extend([25, 27, 29, 31, 33]);
    expect.extend(35..=39);
    expect.extend(45..=59);
    expect.extend((60..=79).flat_map(|k| [k, k]).take(39));
    expect.extend(80..=90);
    assert_eq!(plan, expect);
    assert!(plan.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn a_trim_only_map_reproduces_trim_frame_bounds_exactly() {
    let full = 12_345u32;
    for (i, o, fps) in [
        (0u32, 0u32, 60u64),
        (1, 5000, 60),
        (333, 9999, 30),
        (0, 12_345, 24),
    ] {
        let m = TimeMap::build(
            &Trim {
                in_ms: i,
                out_ms: o,
            },
            &[],
            &[],
            &[],
            full,
        );
        let (in_ms, out_ms) = Trim {
            in_ms: i,
            out_ms: o,
        }
        .resolve(full);
        let (k_in, k_last) = crate::export::pipeline::trim_frame_bounds(in_ms, out_ms, fps);
        assert_eq!(
            m.frame_plan(fps),
            (k_in..=k_last).collect::<Vec<_>>(),
            "trim {i}..{o} @ {fps}"
        );
        assert!(m.is_plain());
    }
    assert!(TimeMap::build(
        &Trim {
            in_ms: 700,
            out_ms: 700
        },
        &[],
        &[],
        &[],
        full
    )
    .frame_plan(60)
    .is_empty());
}

#[test]
fn overlapping_and_touching_cuts_merge_and_a_cut_inside_a_speed_span_wins() {
    let m = TimeMap::build(
        &Trim::default(),
        &[
            Cut {
                id: "a".into(),
                start_ms: 100,
                end_ms: 300,
            },
            Cut {
                id: "b".into(),
                start_ms: 300,
                end_ms: 400,
            },
            Cut {
                id: "c".into(),
                start_ms: 250,
                end_ms: 350,
            },
        ],
        &[Speed {
            id: "s".into(),
            start_ms: 0,
            end_ms: 1000,
            factor: 2.0,
        }],
        &[],
        1000,
    );
    let got: Vec<(u32, u32, f64)> = m
        .segments()
        .iter()
        .map(|s| (s.clip_start, s.clip_end, s.factor))
        .collect();
    assert_eq!(got, vec![(0, 100, 2.0), (400, 1000, 2.0)]);
    assert!(
        m.crosses_boundary(49, 50),
        "output 49 is in (0,100), 50 is in (400,1000): the cut is between"
    );
    assert!(!m.crosses_boundary(10, 20));
}

#[test]
fn speed_spans_are_clamped_against_each_other_and_into_range() {
    let m = TimeMap::build(
        &Trim::default(),
        &[],
        &[
            Speed {
                id: "a".into(),
                start_ms: 100,
                end_ms: 600,
                factor: 40.0,
            },
            Speed {
                id: "b".into(),
                start_ms: 400,
                end_ms: 800,
                factor: 0.01,
            },
        ],
        &[],
        1000,
    );
    let got: Vec<(u32, u32, f64)> = m
        .segments()
        .iter()
        .map(|s| (s.clip_start, s.clip_end, s.factor))
        .collect();
    assert_eq!(
        got,
        vec![
            (0, 100, 1.0),
            (100, 600, 8.0),
            (600, 800, 0.25),
            (800, 1000, 1.0)
        ]
    );
}

#[test]
fn a_cut_covering_everything_leaves_no_segments_and_no_frames() {
    let m = TimeMap::build(
        &Trim::default(),
        &[Cut {
            id: "x".into(),
            start_ms: 0,
            end_ms: 5000,
        }],
        &[],
        &[],
        5000,
    );
    assert!(m.segments().is_empty());
    assert_eq!(m.out_dur_ms(), 0);
    assert!(m.frame_plan(60).is_empty());
    assert_eq!(m.clip_of(0), 0);
    assert!(!m.crosses_boundary(0, 10));
}
