use super::*;
use crate::edit::model::*;
use crate::export::remap::TimeMap;

fn map() -> TimeMap {
    crate::export::remap::tests::fixture()
}

fn spot(id: &str, start_ms: u32, end_ms: u32) -> EffectRegion {
    EffectRegion {
        id: id.into(),
        kind: EffectKind::Spotlight,
        start_ms,
        end_ms,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
        rect: None,
        strength: None,
        roundness: None,
    }
}

#[test]
fn regions_move_to_the_output_clock_and_keep_their_durations() {
    let mut d = EditDoc::default();
    d.zooms.push(Zoom {
        id: "z0".into(),
        start_ms: 2200,
        end_ms: 3200,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "smooth".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 0,
        cam_action: None,
        smart_typing: false,
        easing_out: None,
    });
    let r = remap_doc(&d, &map());
    assert_eq!((r.zooms[0].start_ms, r.zooms[0].end_ms), (700, 1350));
    assert_eq!((r.zooms[0].zoom_in_ms, r.zooms[0].zoom_out_ms), (350, 450));
    assert_eq!(r.zooms[0].id, "z0");
}

#[test]
fn captions_and_their_words_move_onto_the_output_clock() {
    use crate::edit::captions::{Caption, CaptionWord};
    let word = |s, e, t: &str| CaptionWord {
        start_ms: s,
        end_ms: e,
        text: t.into(),
    };
    let mut d = EditDoc::default();
    d.captions.push(Caption {
        id: "c0".into(),
        start_ms: 2200,
        end_ms: 3200,
        text: "hello world".into(),
        words: vec![word(2200, 2600, "hello"), word(2600, 3200, "world")],
    });
    d.captions.push(Caption {
        id: "c1".into(),
        start_ms: 1100,
        end_ms: 1900,
        text: "gone".into(),
        words: vec![],
    });
    let r = remap_doc(&d, &map());
    assert_eq!(
        r.captions.len(),
        1,
        "a caption entirely inside a cut is dropped"
    );
    assert_eq!(
        (
            r.captions[0].id.as_str(),
            r.captions[0].start_ms,
            r.captions[0].end_ms
        ),
        ("c0", 700, 1350)
    );
    let w: Vec<(u32, u32)> = r.captions[0]
        .words
        .iter()
        .map(|w| (w.start_ms, w.end_ms))
        .collect();
    assert_eq!(
        w,
        vec![(700, 1050), (1050, 1350)],
        "each word's own timing moves with the line"
    );
}

#[test]
fn a_region_entirely_inside_a_cut_is_dropped_and_one_straddling_it_shrinks() {
    let mut d = EditDoc::default();
    d.effects.push(spot("e0", 1100, 1900));
    d.effects.push(spot("e1", 800, 2200));
    let r = remap_doc(&d, &map());
    assert_eq!(r.effects.len(), 1);
    assert_eq!(
        (
            r.effects[0].id.as_str(),
            r.effects[0].start_ms,
            r.effects[0].end_ms
        ),
        ("e1", 300, 700)
    );
}

#[test]
fn camera_moves_map_their_time_and_the_consumed_fields_are_cleared() {
    let mut d = EditDoc::default();
    d.trim = Trim {
        in_ms: 500,
        out_ms: 9000,
    };
    d.cuts.push(Cut {
        id: "c0".into(),
        start_ms: 1000,
        end_ms: 2000,
    });
    d.speed.push(Speed {
        id: "s0".into(),
        start_ms: 2500,
        end_ms: 3500,
        factor: 2.0,
    });
    d.camera_moves.push(CameraMove {
        id: "m0".into(),
        t_ms: 1500,
        x: 0.5,
        y: 0.5,
        size: 0.3,
        easing: "smooth".into(),
        shape: "layout".into(),
        roundness: DEFAULT_CAM_ROUNDNESS,
    });
    let r = remap_doc(&d, &map());
    assert_eq!(r.camera_moves[0].t_ms, 500);
    assert_eq!(r.trim, Trim::default());
    assert!(r.cuts.is_empty() && r.speed.is_empty());
    assert_eq!(r.clip_ms, 8500);
}

#[test]
fn a_plain_map_is_the_identity_on_regions() {
    let mut d = EditDoc::default();
    d.layout.push(LayoutSeg {
        id: "l0".into(),
        start_ms: 100,
        end_ms: 900,
        layout: "camera".into(),
        transition_ms: 350,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    });
    let r = remap_doc(&d, &TimeMap::identity(10_000));
    assert_eq!(r.layout, d.layout);
}

#[test]
fn texts_move_to_the_output_clock_and_clips_are_consumed() {
    let mut d = EditDoc::default();
    d.texts.push(crate::edit::text::TextItem {
        id: "t0".into(),
        start_ms: 2200,
        end_ms: 3200,
        kind: Default::default(),
        text: "x".into(),
        sub: None,
        style: "clean".into(),
        pos: Default::default(),
        offset: [0.0, 0.0],
        size: Default::default(),
        anim_in: Default::default(),
        anim_out: Default::default(),
        in_ms: 420,
        out_ms: 420,
        easing: "smooth".into(),
    });
    d.texts.push(crate::edit::text::TextItem {
        id: "t1".into(),
        start_ms: 1100,
        end_ms: 1900,
        ..d.texts[0].clone()
    });
    d.clips.push(crate::edit::clip::Clip {
        id: "cl0".into(),
        src_in_ms: 0,
        src_out_ms: 9000,
        transition_in_ms: 0,
    });
    let r = remap_doc(&d, &map());
    assert_eq!(r.texts.len(), 1, "the text inside the cut is dropped");
    assert_eq!((r.texts[0].start_ms, r.texts[0].end_ms), (700, 1350));
    assert_eq!((r.texts[0].in_ms, r.texts[0].out_ms), (420, 420));
    assert!(r.clips.is_empty());
}
