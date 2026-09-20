use super::{ClipDissolve, ClipMixTrack};
use crate::edit::clip::Clip;
use crate::edit::model::Trim;
use crate::export::remap::TimeMap;
use crate::export::types::Easing;

fn clips(t0: u32, t1: u32) -> Vec<Clip> {
    vec![
        Clip {
            id: "cl1".into(),
            src_in_ms: 6000,
            src_out_ms: 9000,
            transition_in_ms: t0,
        },
        Clip {
            id: "cl0".into(),
            src_in_ms: 500,
            src_out_ms: 4000,
            transition_in_ms: t1,
        },
    ]
}

fn track(t0: u32, t1: u32) -> ClipMixTrack {
    let map = crate::export::remap::clips_tests::clips_fixture();
    let mut t = ClipMixTrack::new(Easing::Linear);
    t.set_clips(clips(t0, t1));
    t.resolve(&map, 10);
    t
}

#[test]
fn a_document_with_no_transition_has_no_dissolve_at_all() {
    let t = track(0, 0);
    assert!(t.is_empty());
    for out_t in [0u32, 4899, 4900, 5000, 6999] {
        assert_eq!(t.at(out_t), None, "out {out_t}");
    }
}

#[test]
fn the_window_opens_on_the_plan_boundary_not_on_the_segments_out_start() {
    let t = track(0, 500);
    assert_eq!(
        t.dissolves(),
        &[ClipDissolve {
            out_start_ms: 4900,
            prev_clip: 0,
            dur_ms: 500,
        }],
        "clip 1 opens at plan index 49, which is 4900 ms, not at its 5000 ms out_start"
    );
    assert_eq!(t.at(4899), None);
    assert_eq!(t.at(5400), None, "the window is half open");
    let m = t.at(4900).expect("the boundary frame is inside the window");
    assert_eq!((m.prev_clip, m.prev_out_ms), (0, 4899));
    assert_eq!(
        m.alpha, 0.0,
        "the boundary frame is entirely the outgoing clip"
    );
}

#[test]
fn the_alpha_is_the_incoming_clips_weight_on_the_documents_curve() {
    let lin = track(0, 500);
    assert_eq!(lin.at(5150).unwrap().alpha, 0.5);
    assert!((lin.at(5399).unwrap().alpha - 0.998).abs() < 1e-6);
    let map = crate::export::remap::clips_tests::clips_fixture();
    let mut smooth = ClipMixTrack::new(Easing::Smooth);
    smooth.set_clips(clips(0, 500));
    smooth.resolve(&map, 10);
    assert!(
        (smooth.at(5025).unwrap().alpha - 0.15625).abs() < 1e-6,
        "0.25 * 0.25 * (3 - 0.5) on the smoothstep the rest of the export dissolves on, where linear would give 0.25"
    );
    assert!((smooth.at(5150).unwrap().alpha - 0.5).abs() < 1e-6);
}

#[test]
fn the_first_clips_transition_is_stored_and_ignored() {
    let t = track(400, 0);
    assert!(t.is_empty(), "nothing dissolves into the first clip");
}

#[test]
fn resolving_again_replaces_the_window_rather_than_appending() {
    let map = crate::export::remap::clips_tests::clips_fixture();
    let mut t = ClipMixTrack::new(Easing::Linear);
    t.set_clips(clips(0, 500));
    t.resolve(&map, 10);
    t.resolve(&map, 10);
    assert_eq!(t.dissolves().len(), 1);
    t.resolve(&map, 60);
    assert_eq!(t.dissolves().len(), 1);
    assert_eq!(
        t.dissolves()[0].out_start_ms,
        map.clip_spans(60)[1].plan_start as u32 * 1000 / 60
    );
}

fn three_clip_track(t1: u32, t2: u32) -> ClipMixTrack {
    let clip = |id: &str, a, b, t| Clip {
        id: id.into(),
        src_in_ms: a,
        src_out_ms: b,
        transition_in_ms: t,
    };
    let list = vec![
        clip("cl0", 0, 3000, 0),
        clip("cl1", 3000, 6000, t1),
        clip("cl2", 6000, 9000, t2),
    ];
    let map = TimeMap::build(&Trim::default(), &[], &[], &list, 9000);
    let mut t = ClipMixTrack::new(Easing::Linear);
    t.set_clips(list);
    t.resolve(&map, 10);
    t
}

#[test]
fn a_transition_longer_than_its_own_clip_is_clamped_to_the_incoming_span() {
    assert_eq!(
        track(0, 5000).dissolves(),
        &[ClipDissolve {
            out_start_ms: 4900,
            prev_clip: 0,
            dur_ms: 2100,
        }],
        "the incoming span is 21 plan entries at 10 fps, so the window is [4900, 7000) and not [4900, 9900)"
    );
    assert_eq!(track(0, 5000).at(5950).unwrap().alpha, 0.5);
    assert_eq!(
        track(0, 5000).at(7000),
        None,
        "the take ends with the window"
    );
    assert_eq!(
        track(0, 500).dissolves()[0].dur_ms,
        500,
        "a transition that fits inside its clip is stored as it is"
    );
}

#[test]
fn an_over_long_window_no_longer_masks_the_next_joins_own_dissolve() {
    let t = three_clip_track(5000, 400);
    assert_eq!(
        t.dissolves(),
        &[
            ClipDissolve {
                out_start_ms: 3000,
                prev_clip: 0,
                dur_ms: 3000,
            },
            ClipDissolve {
                out_start_ms: 6000,
                prev_clip: 1,
                dur_ms: 400,
            },
        ],
        "clip 1 spans 30 plan entries and clip 2 spans 31, so 5000 ms clamps to 3000 and 400 ms stands"
    );
    let m = t.at(6100).expect("6100 is inside the second join's window");
    assert_eq!(
        (m.prev_clip, m.prev_out_ms),
        (1, 5999),
        "unclamped, the first window still held 6100 and answered clip 0"
    );
    assert_eq!(m.alpha, 0.25);
}

#[test]
fn a_document_with_no_clips_never_dissolves() {
    let map = crate::export::remap::tests::fixture();
    let mut t = ClipMixTrack::new(Easing::Smooth);
    t.resolve(&map, 10);
    assert!(t.is_empty());
    assert_eq!(t.at(0), None);
}
