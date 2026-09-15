use crate::edit::model::EditDoc;
use crate::edit::ops::api::{apply, EditOp};
use crate::settings::motion::MotionSettings;

fn doc_with(preset: &str, i: &str, o: &str) -> EditDoc {
    let mut d = EditDoc {
        clip_ms: 60_000,
        ..Default::default()
    };
    d.settings.motion = MotionSettings {
        preset: preset.into(),
        easing: i.into(),
        easing_out: o.into(),
    };
    d
}

#[test]
fn a_new_zoom_inherits_the_project_curve_instead_of_hardcoded_smooth() {
    let mut d = doc_with("mechanical", "linear", "linear");
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    apply(
        &mut d,
        EditOp::AddZoomFull {
            at_ms: 2000,
            dur_ms: 1000,
            scale: 3.0,
        },
    );
    assert_eq!(d.zooms[0].easing, "linear");
    assert_eq!(d.zooms[1].easing, "linear");
    assert_eq!(d.zooms[0].easing_out, None);
}

#[test]
fn a_split_project_curve_stores_the_zooms_out_ramp() {
    let mut d = doc_with("cinematic", "linear", "spring(140,7,1)");
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    assert_eq!(d.zooms[0].easing, "linear");
    assert_eq!(
        d.zooms[0].easing_out.as_deref(),
        Some("spring(140.000,7.000,1.000)")
    );
}

#[test]
fn the_default_project_still_writes_todays_smooth_everywhere() {
    let mut d = EditDoc {
        clip_ms: 60_000,
        ..Default::default()
    };
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    apply(
        &mut d,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    apply(
        &mut d,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(
        (d.zooms[0].easing.as_str(), d.zooms[0].easing_out.as_deref()),
        ("smooth", None)
    );
    assert_eq!(
        (d.layout[0].easing.as_str(), d.layout[0].easing_out.as_str()),
        ("smooth", "smooth")
    );
    assert_eq!(d.camera_moves[0].easing, "smooth");
}

#[test]
fn a_layout_seg_and_a_camera_move_inherit_the_project_curve() {
    let mut d = doc_with("mechanical", "linear", "ease_out");
    apply(
        &mut d,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    apply(
        &mut d,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    assert_eq!(
        (d.layout[0].easing.as_str(), d.layout[0].easing_out.as_str()),
        ("linear", "ease_out")
    );
    assert_eq!(d.camera_moves[0].easing, "linear");
}

#[test]
fn an_explicit_layout_easing_out_still_beats_the_project_default() {
    let mut d = doc_with("mechanical", "linear", "ease_out");
    apply(
        &mut d,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "camera".into(),
            transition_out_ms: Some(200),
            easing_out: Some("ease_in".into()),
        },
    );
    assert_eq!(d.layout[0].easing_out, "ease_in");
}

fn update(id: &str, easing: Option<String>, easing_out: Option<String>) -> EditOp {
    EditOp::UpdateZoom {
        id: id.into(),
        start_ms: None,
        end_ms: None,
        scale: None,
        target: None,
        easing,
        zoom_in_ms: None,
        zoom_out_ms: None,
        layer: None,
        smart_typing: None,
        easing_out,
    }
}

#[test]
fn update_zoom_sets_clears_and_collapses_the_out_ramp() {
    let mut d = EditDoc {
        clip_ms: 60_000,
        ..Default::default()
    };
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    let id = d.zooms[0].id.clone();
    apply(&mut d, update(&id, None, Some("linear".into())));
    assert_eq!(d.zooms[0].easing_out.as_deref(), Some("linear"));
    apply(&mut d, update(&id, None, Some("smooth".into())));
    assert_eq!(d.zooms[0].easing_out, None);
    apply(&mut d, update(&id, None, Some("linear".into())));
    apply(&mut d, update(&id, None, Some(String::new())));
    assert_eq!(d.zooms[0].easing_out, None);
    apply(&mut d, update(&id, None, Some("ease_in".into())));
    apply(&mut d, update(&id, Some("ease_out".into()), None));
    assert_eq!(
        (d.zooms[0].easing.as_str(), d.zooms[0].easing_out.as_deref()),
        ("ease_out", Some("ease_in"))
    );
}

#[test]
fn apply_motion_default_stamps_every_region_as_one_step() {
    let mut d = doc_with("mechanical", "linear", "ease_out");
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    apply(
        &mut d,
        EditOp::AddLayoutSeg {
            at_ms: 0,
            dur_ms: 1000,
            layout: "camera".into(),
            transition_out_ms: None,
            easing_out: None,
        },
    );
    apply(
        &mut d,
        EditOp::AddCameraMove {
            t_ms: 500,
            x: 0.5,
            y: 0.5,
            size: 0.3,
            shape: None,
            roundness: None,
        },
    );
    let id = d.zooms[0].id.clone();
    apply(
        &mut d,
        update(&id, Some("spring".into()), Some("smooth".into())),
    );
    d.settings.motion = MotionSettings {
        preset: "bouncy".into(),
        easing: "spring(140,7,1)".into(),
        easing_out: "spring(140,7,1)".into(),
    };
    apply(&mut d, EditOp::ApplyMotionDefault);
    let want = "spring(140.000,7.000,1.000)";
    assert_eq!(d.zooms[0].easing, want);
    assert_eq!(d.zooms[0].easing_out, None);
    assert_eq!(
        (d.layout[0].easing.as_str(), d.layout[0].easing_out.as_str()),
        (want, want)
    );
    assert_eq!(d.camera_moves[0].easing, want);
    assert_eq!((d.zooms[0].zoom_in_ms, d.zooms[0].zoom_out_ms), (350, 450));
}

#[test]
fn a_snappy_preset_string_survives_valid_easing() {
    let snappy = "keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)";
    let mut d = doc_with("snappy", snappy, snappy);
    apply(
        &mut d,
        EditOp::AddZoom {
            at_ms: 0,
            dur_ms: 1000,
        },
    );
    assert_eq!(
        d.zooms[0].easing,
        "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)"
    );
}
