use super::{at_instants, latched_ms, PreviewAt};
use crate::edit::clip::Clip;
use crate::edit::model::{Cut, Trim};
use crate::export::remap::TimeMap;
use crate::export::render::OUT_FPS;

#[test]
fn a_clip_instant_resolves_the_way_it_always_has() {
    let m = crate::export::remap::tests::fixture();
    for t in [500u32, 900, 2100, 3000, 4600, 8000] {
        assert_eq!(
            at_instants(&m, PreviewAt::Clip(t)),
            (m.out_of(t), t),
            "clip {t}"
        );
    }
}

#[test]
fn an_output_instant_round_trips_inside_a_segment() {
    let m = crate::export::remap::tests::fixture();
    for t in [500u32, 900, 2100, 3000, 4600, 8000] {
        let out = m.out_of(t);
        assert_eq!(
            at_instants(&m, PreviewAt::Out(out)),
            (out, t),
            "clip {t} -> out {out}"
        );
    }
}

#[test]
fn an_output_instant_picks_the_showing_the_playhead_is_actually_in() {
    let m = crate::export::remap::clips_tests::clips_fixture();
    assert_eq!(at_instants(&m, PreviewAt::Out(0)), (0, 6000));
    assert_eq!(at_instants(&m, PreviewAt::Out(5250)), (5250, 750));
    assert_eq!(
        at_instants(&m, PreviewAt::Clip(750)),
        (5250, 750),
        "the clip form still answers with the first showing"
    );
}

#[test]
fn the_outgoing_frame_is_the_one_the_export_latched_not_the_one_after_it() {
    let m = crate::export::remap::clips_tests::clips_fixture();
    let opens_at = m.clip_spans(10)[1].plan_start;
    let prev_out_ms = (opens_at as u64 * 1000 / 10) as u32 - 1;
    assert_eq!((opens_at, prev_out_ms), (49, 4899));
    assert_eq!(m.frame_plan(10)[48], 89, "j_prev = 4899 * 10 / 1000 = 48");
    assert_eq!(latched_ms(&m, prev_out_ms, 10), Some(8900));

    let clip = |id: &str, a, b| Clip {
        id: id.into(),
        src_in_ms: a,
        src_out_ms: b,
        transition_in_ms: 0,
    };
    let split = TimeMap::build(
        &Trim::default(),
        &[],
        &[],
        &[clip("a", 0, 1210), clip("b", 1210, 2507)],
        2507,
    );
    let opens_at = split.clip_spans(60)[1].plan_start;
    let prev_out_ms = (opens_at as u64 * 1000 / 60) as u32 - 1;
    assert_eq!((opens_at, prev_out_ms), (73, 1215));
    assert_eq!(
        split.frame_plan(60)[72],
        72,
        "j_prev = 1215 * 60 / 1000 = 72"
    );
    assert_eq!(latched_ms(&split, prev_out_ms, 60), Some(1200));
    assert_eq!(
        split.clip_of(prev_out_ms),
        1215,
        "clip_of would seek past frame 72 at 1200 ms into frame 73 at 1216 ms"
    );
}

#[test]
fn an_empty_plan_latches_no_outgoing_frame() {
    let all_cut = TimeMap::build(
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
    assert!(all_cut.frame_plan(OUT_FPS).is_empty());
    assert_eq!(latched_ms(&all_cut, 0, OUT_FPS), None);
}

#[test]
fn the_paused_dissolve_decodes_the_latched_frame_and_blends_it_at_the_incoming_weight() {
    let src = include_str!("compose.rs");
    let at = |from: usize, needle: &str| {
        src[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} is gone from the dissolve"))
    };
    let d = at(0, "fn clip_dissolve");
    let (latched, decode, blend) = (
        at(d, "latched_ms("),
        at(d, "decode_screen("),
        at(d, "blend_into("),
    );
    assert!(
        latched < decode && decode < blend,
        "clip_dissolve must find the latched instant, decode it, then blend it"
    );
    let body = &src[d..d + at(d, "\n}")];
    assert!(
        !body.contains("clip_of("),
        "B4-R16: clip_of(prev_out_ms) lands on the frame AFTER the export's latch"
    );
    assert!(
        body.contains("m.alpha"),
        "the blend runs on the incoming clip's weight"
    );
    assert!(
        src[at(0, "fn composite_frame")..].contains("clip_dissolve("),
        "composite_frame must run the dissolve"
    );
}
