use super::*;
use std::path::PathBuf;

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

fn sample_doc() -> EditDoc {
    EditDoc {
        version: 1,
        trim: Trim {
            in_ms: 100,
            out_ms: 5000,
        },
        clip_ms: 5000,
        cuts: vec![Cut {
            id: "c0".into(),
            start_ms: 500,
            end_ms: 1000,
        }],
        zooms: vec![Zoom {
            id: "z1".into(),
            start_ms: 200,
            end_ms: 800,
            target: ZoomTarget::Cursor,
            scale: 2.2,
            easing: "ease".into(),
            zoom_in_ms: 350,
            zoom_out_ms: 450,
            layer: 0,
            cam_action: None,
            smart_typing: false,
            easing_out: None,
        }],
        speed: vec![Speed {
            id: "s1".into(),
            start_ms: 1000,
            end_ms: 2000,
            factor: 2.0,
        }],
        layout: vec![LayoutSeg {
            id: "l1".into(),
            start_ms: 0,
            end_ms: 5000,
            layout: "screen".into(),
            transition_ms: 350,
            easing: "smooth".into(),
            transition_out_ms: 0,
            easing_out: "smooth".into(),
            arrangement: None,
        }],
        effects: vec![],
        camera_moves: vec![],
        aspect: crate::export::types::Aspect::default(),
        settings: crate::settings::model::Settings::default(),
        captions: vec![],
        texts: vec![],
        clips: vec![],
    }
}

#[test]
fn zoom_and_effect_region_layer_defaults_to_zero_on_missing_field() {
    let zoom_json = r#"{"id":"z0","start_ms":0,"end_ms":1000,"target":"cursor","scale":2.0,"easing":"smooth","zoom_in_ms":350,"zoom_out_ms":450}"#;
    let zoom: Zoom = serde_json::from_str(zoom_json).unwrap();
    assert_eq!(zoom.layer, 0);

    let effect_json = r#"{"id":"e0","kind":"spotlight","start_ms":0,"end_ms":1000,"fade_in_ms":250,"fade_out_ms":250}"#;
    let effect: EffectRegion = serde_json::from_str(effect_json).unwrap();
    assert_eq!(effect.layer, 0);
}

#[test]
fn layout_seg_exit_transition_defaults_to_a_hard_cut_on_missing_fields() {
    let json = r#"{"id":"l0","start_ms":0,"end_ms":1000,"layout":"camera","transition_ms":350,"easing":"smooth"}"#;
    let s: LayoutSeg = serde_json::from_str(json).unwrap();
    assert_eq!(s.transition_out_ms, 0);
    assert_eq!(s.easing_out, "smooth");
    let doc_json = format!(
        r#"{{"version":2,"trim":{{"in_ms":0,"out_ms":5000}},"cuts":[],"zooms":[],"speed":[],"layout":[{json}],"settings":{{}}}}"#
    );
    let doc: EditDoc = serde_json::from_str(&doc_json).unwrap();
    assert_eq!(
        doc.layout[0],
        LayoutSeg {
            id: "l0".into(),
            start_ms: 0,
            end_ms: 1000,
            layout: "camera".into(),
            transition_ms: 350,
            easing: "smooth".into(),
            transition_out_ms: 0,
            easing_out: "smooth".into(),
            arrangement: None
        }
    );
}

#[test]
fn round_trip_save_load() {
    let doc = sample_doc();
    let p = tmp_path("edit_model_round_trip.json");
    doc.save(&p).unwrap();
    let loaded = EditDoc::load(&p).unwrap();
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.trim.in_ms, 100);
    assert_eq!(loaded.cuts.len(), 1);
    assert_eq!(loaded.zooms.len(), 1);
    assert_eq!(loaded.zooms[0].id, "z1");
    assert_eq!(loaded.speed.len(), 1);
    assert_eq!(loaded.layout.len(), 1);
    assert_eq!(loaded, doc);
}

#[test]
fn load_missing_path_is_none() {
    let p = tmp_path("edit_model_no_such_file_xyz.json");
    let _ = std::fs::remove_file(&p);
    assert!(EditDoc::load(&p).is_none());
}

#[test]
fn partial_json_fills_defaults() {
    let json = r#"{"zooms":[{"id":"z1","start_ms":0,"end_ms":100,"target":"cursor","scale":2.0,"easing":"linear"}]}"#;
    let doc: EditDoc = serde_json::from_str(json).unwrap();
    assert_eq!(doc.version, 1);
    assert_eq!(doc.cuts.len(), 0);
    assert_eq!(doc.speed.len(), 0);
    assert_eq!(doc.layout.len(), 0);
    assert_eq!(doc.trim, Trim::default());
    assert_eq!(doc.zooms.len(), 1);
}

#[test]
fn old_json_without_durations_gets_tuned_defaults() {
    let json = r#"{"zooms":[{"id":"z0","start_ms":0,"end_ms":100,"target":"cursor","scale":2.0,"easing":"smooth"}],"effects":[{"id":"e0","kind":"spotlight","start_ms":0,"end_ms":100}]}"#;
    let doc: EditDoc = serde_json::from_str(json).unwrap();
    assert_eq!(
        (doc.zooms[0].zoom_in_ms, doc.zooms[0].zoom_out_ms),
        (350, 450)
    );
    assert_eq!(
        (doc.effects[0].fade_in_ms, doc.effects[0].fade_out_ms),
        (250, 250)
    );
}

#[test]
fn zoom_target_fixed_serializes_with_xy() {
    let t = ZoomTarget::Fixed { x: 0.5, y: 0.75 };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"x\""), "x missing: {}", json);
    assert!(json.contains("\"y\""), "y missing: {}", json);
    assert!(json.contains("fixed"), "variant missing: {}", json);
    let back: ZoomTarget = serde_json::from_str(&json).unwrap();
    assert_eq!(back, t);
}

#[test]
fn camera_move_round_trip_save_load() {
    let mut doc = sample_doc();
    doc.camera_moves = vec![CameraMove {
        id: "k1".into(),
        t_ms: 300,
        x: 0.5,
        y: 0.4,
        size: 0.3,
        easing: "smooth".into(),
        shape: "layout".into(),
        roundness: DEFAULT_CAM_ROUNDNESS,
    }];
    let p = tmp_path("edit_model_camera_move_round_trip.json");
    doc.save(&p).unwrap();
    let loaded = EditDoc::load(&p).unwrap();
    assert_eq!(loaded.camera_moves.len(), 1);
    assert_eq!(loaded, doc);
}

#[test]
fn zoom_cam_action_defaults_to_none_and_is_omitted_when_unset() {
    let json =
        r#"{"id":"z0","start_ms":0,"end_ms":100,"target":"cursor","scale":2.0,"easing":"smooth"}"#;
    let z: Zoom = serde_json::from_str(json).unwrap();
    assert_eq!(z.cam_action, None);
    let out = serde_json::to_string(&z).unwrap();
    assert!(
        !out.contains("cam_action"),
        "unset action must not be written: {}",
        out
    );
}

#[test]
fn zoom_cam_action_round_trips_when_set() {
    use crate::settings::model::CamZoomAction;
    let mut z: Zoom = serde_json::from_str(
        r#"{"id":"z0","start_ms":0,"end_ms":100,"target":"cursor","scale":2.0,"easing":"smooth"}"#,
    )
    .unwrap();
    z.cam_action = Some(CamZoomAction::Shrink { to: 0.4 });
    let back: Zoom = serde_json::from_str(&serde_json::to_string(&z).unwrap()).unwrap();
    assert_eq!(back.cam_action, Some(CamZoomAction::Shrink { to: 0.4 }));
}

#[test]
fn camera_move_missing_field_defaults_to_empty_vec() {
    let doc: EditDoc = serde_json::from_str(r#"{"zooms":[]}"#).unwrap();
    assert_eq!(doc.camera_moves.len(), 0);
}

#[test]
fn aspect_missing_field_defaults_to_source() {
    use crate::export::types::Aspect;
    let doc: EditDoc = serde_json::from_str(r#"{"zooms":[]}"#).unwrap();
    assert_eq!(doc.aspect, Aspect::Source);
    assert_eq!(EditDoc::default().aspect, Aspect::Source);
}

#[test]
fn aspect_round_trips_through_json() {
    use crate::export::types::Aspect;
    let mut doc = EditDoc::default();
    doc.aspect = Aspect::Square1x1;
    let back: EditDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.aspect, Aspect::Square1x1);
}

#[test]
fn default_trim_resolves_to_the_whole_clip() {
    assert_eq!(Trim::default().resolve(12_345), (0, 12_345));
}

#[test]
fn trim_resolve_clamps_in_to_out_and_both_to_the_clip() {
    assert_eq!(
        Trim {
            in_ms: 2_000,
            out_ms: 999_999
        }
        .resolve(10_000),
        (2_000, 10_000)
    );
    assert_eq!(
        Trim {
            in_ms: 9_000,
            out_ms: 5_000
        }
        .resolve(10_000),
        (5_000, 5_000)
    );
}

#[test]
fn clip_ms_missing_field_defaults_to_zero() {
    let doc: EditDoc = serde_json::from_str(r#"{"zooms":[]}"#).unwrap();
    assert_eq!(doc.clip_ms, 0);
}

#[path = "model_save_tests.rs"]
mod save_tests;

#[path = "model_arrangement_tests.rs"]
mod arrangement_tests;

#[cfg(test)]
#[path = "model_roundtrip_tests.rs"]
mod roundtrip_tests;
