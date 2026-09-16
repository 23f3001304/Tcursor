use crate::edit::clip::Clip;
use crate::edit::model::*;
use crate::edit::text::{TextAnchor, TextAnim, TextItem, TextKind, TextSize};

const LEGACY: &str = r#"{"version":1,"trim":{"in_ms":100,"out_ms":5000},"cuts":[{"start_ms":500,"end_ms":1000}],
"zooms":[{"id":"z0","start_ms":200,"end_ms":800,"target":"cursor","scale":2.2,"easing":"smooth"}],
"speed":[],"layout":[],"settings":{"zoom":{"target_scale":2.0}}}"#;

fn plain() -> EditDoc {
    let mut d = EditDoc::default();
    d.clip_ms = 5000;
    d
}

fn full_house() -> EditDoc {
    let mut d = EditDoc::default();
    d.clip_ms = 10_000;
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
    d.zooms.push(Zoom {
        id: "z0".into(),
        start_ms: 200,
        end_ms: 800,
        target: ZoomTarget::Fixed { x: 0.3, y: 0.4 },
        scale: 2.2,
        easing: "spring(300.000,10.000,1.000)".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 1,
        cam_action: Some(crate::settings::model::CamZoomAction::Hide),
        smart_typing: true,
        easing_out: Some("ease_out".into()),
    });
    d.layout.push(LayoutSeg {
        id: "l0".into(),
        start_ms: 0,
        end_ms: 5000,
        layout: "presenter".into(),
        transition_ms: 350,
        easing: "smooth".into(),
        transition_out_ms: 200,
        easing_out: "smooth".into(),
        arrangement: Some(Arrangement {
            screen: Some(PanelPose {
                cx: 0.5,
                cy: 0.5,
                size: 0.8,
            }),
            cam: None,
        }),
    });
    d.layout.push(LayoutSeg {
        id: "l1".into(),
        start_ms: 5000,
        end_ms: 9000,
        layout: "screen".into(),
        transition_ms: 350,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    });
    let mut mask = EffectRegion {
        id: "e0".into(),
        kind: EffectKind::Blur,
        start_ms: 100,
        end_ms: 900,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: Some(0.02),
        layer: 0,
        rect: Some([0.35, 0.40, 0.30, 0.20]),
        strength: Some(0.03),
        roundness: Some(0.1),
    };
    d.effects.push(mask.clone());
    mask.id = "e1".into();
    mask.kind = EffectKind::Spotlight;
    mask.mode = Some(crate::settings::model::SpotlightMode::Halo);
    mask.rect = None;
    mask.feather = None;
    mask.strength = None;
    mask.roundness = None;
    mask.dim = Some(0.3);
    mask.radius = Some(0.2);
    d.effects.push(mask);
    d.camera_moves.push(CameraMove {
        id: "k0".into(),
        t_ms: 1200,
        x: 0.8,
        y: 0.8,
        size: 0.25,
        easing: "smooth".into(),
        shape: "circle".into(),
        roundness: DEFAULT_CAM_ROUNDNESS,
    });
    d.captions.push(crate::edit::captions::Caption {
        id: "c0".into(),
        start_ms: 1000,
        end_ms: 3000,
        text: "hello world".into(),
        words: vec![crate::edit::captions::CaptionWord {
            start_ms: 1000,
            end_ms: 1500,
            text: "hello".into(),
        }],
    });
    d.texts.push(TextItem {
        id: "t0".into(),
        start_ms: 500,
        end_ms: 3500,
        kind: TextKind::LowerThird,
        text: "Name".into(),
        sub: Some("Role".into()),
        style: "bar".into(),
        pos: TextAnchor::BottomLeft,
        offset: [0.04, -0.06],
        size: TextSize::L,
        anim_in: TextAnim::Slide,
        anim_out: TextAnim::Pop,
        in_ms: 300,
        out_ms: 500,
        easing: "smooth".into(),
    });
    d.texts.push(TextItem {
        id: "t1".into(),
        start_ms: 4000,
        end_ms: 6000,
        kind: TextKind::Title,
        text: "Hello".into(),
        sub: None,
        style: "clean".into(),
        pos: TextAnchor::MidCenter,
        offset: [0.0, 0.0],
        size: TextSize::L,
        anim_in: TextAnim::Fade,
        anim_out: TextAnim::Fade,
        in_ms: 420,
        out_ms: 420,
        easing: "smooth".into(),
    });
    d.clips.push(Clip {
        id: "cl1".into(),
        src_in_ms: 6000,
        src_out_ms: 9000,
        transition_in_ms: 0,
    });
    d.clips.push(Clip {
        id: "cl0".into(),
        src_in_ms: 500,
        src_out_ms: 4000,
        transition_in_ms: 500,
    });
    d.settings.grade = crate::settings::grade::GradeSettings {
        preset: crate::settings::grade::GradePreset::Cinematic,
        exposure: 0.0,
        contrast: 1.12,
        vignette: 0.28,
    };
    d
}

fn pretty(d: &EditDoc) -> Vec<u8> {
    serde_json::to_vec_pretty(d).unwrap()
}

#[test]
fn a_plain_doc_a_full_house_and_a_migrated_legacy_doc_round_trip_byte_identically() {
    let legacy: EditDoc = serde_json::from_str(LEGACY).unwrap();
    for (name, doc) in [
        ("plain", plain()),
        ("full", full_house()),
        ("legacy", legacy),
    ] {
        let once = pretty(&doc);
        let back: EditDoc = serde_json::from_slice(&once).unwrap();
        assert_eq!(back, doc, "{name}: parse(save(doc)) == doc");
        assert_eq!(
            pretty(&back),
            once,
            "{name}: save(parse(save(doc))) == save(doc)"
        );
    }
}

#[test]
fn the_legacy_doc_lands_on_every_default_the_new_fields_have() {
    let d: EditDoc = serde_json::from_str(LEGACY).unwrap();
    assert!(
        d.texts.is_empty() && d.clips.is_empty() && d.effects.is_empty() && d.captions.is_empty()
    );
    assert!(d.settings.grade.is_identity());
    assert_eq!(d.settings.zoom.target_scale, 2.0);
    assert_eq!(
        d.version, 1,
        "loading never bumps the version; migrate.rs does"
    );
}

#[test]
fn the_new_fields_are_always_written_so_the_frontend_never_sees_undefined() {
    let s = String::from_utf8(pretty(&plain())).unwrap();
    for key in [
        "\"texts\"",
        "\"clips\"",
        "\"grade\"",
        "\"captions\"",
        "\"effects\"",
    ] {
        assert!(s.contains(key), "{key} missing from a saved plain doc");
    }
}
