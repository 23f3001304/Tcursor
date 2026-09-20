use super::*;

#[test]
fn atempo_chains_stay_inside_ffmpegs_classic_range() {
    assert_eq!(atempo_chain(1.0), "");
    assert_eq!(atempo_chain(2.0), "atempo=2");
    assert_eq!(atempo_chain(4.0), "atempo=2,atempo=2");
    assert_eq!(atempo_chain(3.0), "atempo=2,atempo=1.5");
    assert_eq!(atempo_chain(0.25), "atempo=0.5,atempo=0.5");
    assert_eq!(atempo_chain(8.0), "atempo=2,atempo=2,atempo=2");
}

#[test]
fn the_identity_emits_no_chain_so_the_mux_command_is_unchanged() {
    assert_eq!(
        segment_chain(
            "[x]",
            &[AudioSeg {
                start_s: 0.0,
                end_s: 8.5,
                factor: 1.0
            }],
            "[a]"
        ),
        None
    );
    assert_eq!(segment_chain("[x]", &[], "[a]"), None);
}

#[test]
fn one_retimed_segment_needs_no_split_or_concat() {
    assert_eq!(
        segment_chain(
            "[x]",
            &[AudioSeg {
                start_s: 0.0,
                end_s: 2.0,
                factor: 2.0
            }],
            "[a]"
        )
        .unwrap(),
        "[x]atrim=start=0.000:end=2.000,asetpts=PTS-STARTPTS,atempo=2[a]"
    );
}

#[test]
fn audio_segs_come_from_the_map_in_trimmed_video_seconds() {
    let m = crate::export::remap::tests::fixture();
    let segs = audio_segs(&m, 10, 500);
    assert_eq!(segs.len(), 7);
    assert_eq!(
        (segs[0].start_s, segs[0].end_s, segs[0].factor),
        (0.0, 0.5, 1.0)
    );
    assert_eq!(
        (segs[2].start_s, segs[2].end_s, segs[2].factor),
        (2.0, 3.0, 2.0)
    );
    assert_eq!(
        (segs[6].start_s, segs[6].end_s, segs[6].factor),
        (7.5, 8.5, 1.0)
    );
}

#[test]
fn the_origin_is_the_earliest_source_frame_the_plan_uses() {
    let base = crate::export::remap::tests::fixture();
    let plan = base.frame_plan(10);
    assert_eq!(audio_origin_q(&plan, 10), 500);
    assert_eq!(
        audio_origin_q(&plan, 10),
        plan[0] * 1000 / 10,
        "monotone: the first entry"
    );
    let clips = crate::export::remap::clips_tests::clips_fixture();
    let cplan = clips.frame_plan(10);
    assert_eq!(cplan[0], 60, "the reordered plan opens on the later clip");
    assert_eq!(
        audio_origin_q(&cplan, 10),
        500,
        "but the origin is the earliest frame"
    );
    assert_eq!(audio_origin_q(&[], 10), 0);
}

#[test]
fn a_reordered_map_trims_audio_in_output_order_and_never_asks_for_a_negative_start() {
    let m = crate::export::remap::clips_tests::clips_fixture();
    let segs = audio_segs(&m, 10, audio_origin_q(&m.frame_plan(10), 10));
    let got: Vec<(f64, f64, f64)> = segs
        .iter()
        .map(|s| (s.start_s, s.end_s, s.factor))
        .collect();
    assert_eq!(
        got,
        vec![
            (5.5, 7.5, 0.5),
            (7.5, 8.5, 1.0),
            (0.0, 0.5, 1.0),
            (1.5, 2.0, 1.0),
            (2.0, 3.0, 2.0),
            (3.0, 3.5, 1.0),
        ]
    );
    assert!(segs.iter().all(|s| s.start_s >= 0.0));
}

#[test]
fn a_clip_whose_first_frame_ceils_above_the_origin_is_clamped_rather_than_negative() {
    use crate::edit::clip::Clip;
    use crate::edit::model::Trim;
    let clip = |id: &str, a, b| Clip {
        id: id.into(),
        src_in_ms: a,
        src_out_ms: b,
        transition_in_ms: 0,
    };
    let m = crate::export::remap::TimeMap::build(
        &Trim::default(),
        &[],
        &[],
        &[clip("cl0", 4000, 9000), clip("cl1", 505, 4000)],
        10_000,
    );
    let plan = m.frame_plan(10);
    assert_eq!(
        plan.iter().copied().min(),
        Some(6),
        "505 ms ceils to frame 6"
    );
    let origin = audio_origin_q(&plan, 10);
    assert_eq!(
        origin, 600,
        "which is 600 ms, 95 ms after the clip's own start"
    );
    let segs = audio_segs(&m, 10, origin);
    let got: Vec<(f64, f64)> = segs.iter().map(|s| (s.start_s, s.end_s)).collect();
    assert_eq!(
        got,
        vec![(3.4, 8.4), (0.0, 3.4)],
        "clamped at zero, not -0.095"
    );
}

#[test]
fn every_branch_of_a_multi_segment_chain_ramps_in_and_out() {
    let segs = [
        AudioSeg {
            start_s: 0.0,
            end_s: 0.5,
            factor: 1.0,
        },
        AudioSeg {
            start_s: 1.5,
            end_s: 2.5,
            factor: 2.0,
        },
    ];
    assert_eq!(segment_chain("[x]", &segs, "[a]").unwrap(),
        "[x]asplit=2[x0][x1];[x0]atrim=start=0.000:end=0.500,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=0.020,afade=t=out:st=0.480:d=0.020[s0];[x1]atrim=start=1.500:end=2.500,asetpts=PTS-STARTPTS,atempo=2,afade=t=in:st=0:d=0.020,afade=t=out:st=0.480:d=0.020[s1];[s0][s1]concat=n=2:v=0:a=1[a]");
}

#[test]
fn a_branch_shorter_than_two_ramps_halves_them_rather_than_overlapping() {
    let segs = [
        AudioSeg {
            start_s: 0.0,
            end_s: 0.020,
            factor: 1.0,
        },
        AudioSeg {
            start_s: 1.0,
            end_s: 2.0,
            factor: 1.0,
        },
    ];
    let chain = segment_chain("[x]", &segs, "[a]").unwrap();
    assert!(
        chain.contains("afade=t=in:st=0:d=0.010,afade=t=out:st=0.010:d=0.010"),
        "{chain}"
    );
}
