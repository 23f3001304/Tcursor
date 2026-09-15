use super::*;
use crate::edit::model::{EffectKind, ZoomTarget};
use crate::edit::ops::api::EditOp;

const CLIP: u32 = 20_000;
fn one_click() -> Vec<ClickAt> {
    vec![ClickAt {
        t_ms: 3_000,
        x: 0.50,
        y: 0.50,
    }]
}

#[test]
fn a_clean_plan_maps_to_ops_in_time_order() {
    let raw = r#"{"edits":[
      {"kind":"zoom","at_ms":8000,"dur_ms":1500,"rect":[0.30,0.20,0.20,0.16],"why":"the settings dialog opens"},
      {"kind":"layout","at_ms":12000,"dur_ms":4000,"layout":"presenter","why":"you turn to the camera"},
      {"kind":"trim","in_ms":1200,"out_ms":0,"why":"dead air before you start"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 3);
    assert_eq!(
        p.iter().map(|x| x.kind).collect::<Vec<_>>(),
        vec![ProposalKind::Trim, ProposalKind::Zoom, ProposalKind::Layout],
        "sorted by at_ms: a trim sits at 0"
    );
    assert_eq!(p[0].id, "p0");
    let zoom = p.iter().find(|x| x.kind == ProposalKind::Zoom).unwrap();
    assert!(matches!(
        zoom.ops[0],
        EditOp::AddZoomFull {
            at_ms: 8_000,
            dur_ms: 1_500,
            ..
        }
    ));
    assert!(
        matches!(&zoom.ops[1],
        EditOp::UpdateZoom { id, target: Some(ZoomTarget::Fixed { x, y }), .. }
        if id == NEW_ID && (*x - 0.40).abs() < 1e-4 && (*y - 0.28).abs() < 1e-4),
        "the rect's centre becomes the aim point: {:?}",
        zoom.ops[1]
    );
}

#[test]
fn one_unparseable_item_is_dropped_and_its_siblings_survive() {
    let raw = r#"{"edits":[
      {"kind":"zoom","at_ms":"three thousand","dur_ms":1500,"why":"bad types"},
      {"kind":"zoom","at_ms":6000,"dur_ms":1200,"why":"good one"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1, "per item, not per run: {:?}", p);
    assert_eq!(p[0].why, "good one");
}

#[test]
fn garbage_is_an_empty_sheet_rather_than_an_error() {
    assert!(proposals_from_json("I could not find anything to edit.", CLIP, &[]).is_empty());
    assert!(proposals_from_json(r#"{"edits":[]}"#, CLIP, &[]).is_empty());
    assert!(proposals_from_json("", CLIP, &[]).is_empty());
}

#[test]
fn a_fenced_reply_with_a_brace_inside_a_why_string_still_parses() {
    let raw = "```json\n{\"edits\":[{\"kind\":\"zoom\",\"at_ms\":500,\"dur_ms\":1000,\"why\":\"the idle stretch } at the start\"}]}\n```";
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1);
    assert!(p[0].why.contains('}'));
}

#[test]
fn an_edit_running_past_the_clip_end_is_dropped_and_the_rest_stand() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":19900,"dur_ms":2000,"why":"past the end"},
                           {"kind":"zoom","at_ms":1000,"dur_ms":1000,"why":"fine"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].at_ms, 1_000);
}

#[test]
fn an_adversarial_duration_drops_the_item_instead_of_overflowing() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":100,"dur_ms":4294967295,"why":"overflow"}]}"#;
    assert!(proposals_from_json(raw, CLIP, &[]).is_empty());
}

#[test]
fn a_rect_outside_the_frame_leaves_a_cursor_following_zoom_rather_than_dropping_it() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":1000,"dur_ms":1200,"rect":[0.8,0.9,0.5,0.5],"why":"off frame"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1);
    assert_eq!(
        p[0].ops.len(),
        1,
        "no aim op, but the zoom itself is still worth proposing"
    );
    assert!(
        p[0].rect.is_none(),
        "and the sheet must not outline a rect we refused"
    );
}

#[test]
fn a_zoom_within_a_second_of_a_click_snaps_to_it_and_borrows_its_point() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":3600,"dur_ms":1500,"why":"you click Save"}]}"#;
    let p = proposals_from_json(raw, CLIP, &one_click());
    assert_eq!(p[0].at_ms, 3_000 - PRE_ROLL_MS);
    assert!(matches!(&p[0].ops[1],
        EditOp::UpdateZoom { target: Some(ZoomTarget::Fixed { x, y }), .. }
        if (*x - 0.5).abs() < 1e-4 && (*y - 0.5).abs() < 1e-4));
}

#[test]
fn a_zoom_with_no_click_within_a_second_keeps_the_models_own_timing() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":12000,"dur_ms":1500,"why":"a slow reveal"}]}"#;
    let p = proposals_from_json(raw, CLIP, &one_click());
    assert_eq!(p[0].at_ms, 12_000);
    assert_eq!(p[0].ops.len(), 1);
}

#[test]
fn the_models_own_rect_beats_the_snapped_clicks_point() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":3200,"dur_ms":1500,"rect":[0.0,0.0,0.2,0.2],"why":"the dialog, not the button"}]}"#;
    let p = proposals_from_json(raw, CLIP, &one_click());
    assert_eq!(p[0].at_ms, 3_000 - PRE_ROLL_MS, "the time still snaps");
    assert!(matches!(&p[0].ops[1],
        EditOp::UpdateZoom { target: Some(ZoomTarget::Fixed { x, .. }), .. } if (*x - 0.1).abs() < 1e-4));
}

#[test]
fn scale_comes_from_how_much_of_the_frame_the_rect_covers_and_is_clamped_both_ways() {
    let s = |raw: &str| match proposals_from_json(raw, CLIP, &[])[0].ops[0] {
        EditOp::AddZoomFull { scale, .. } => scale,
        ref o => panic!("{:?}", o),
    };
    let tight = r#"{"edits":[{"kind":"zoom","at_ms":0,"dur_ms":1000,"rect":[0.4,0.4,0.03,0.03],"why":"tiny"}]}"#;
    let wide = r#"{"edits":[{"kind":"zoom","at_ms":0,"dur_ms":1000,"rect":[0.0,0.0,0.9,0.9],"why":"almost everything"}]}"#;
    assert!((s(tight) - MAX_SCALE).abs() < 1e-4);
    assert!((s(wide) - MIN_SCALE).abs() < 1e-4);
}

#[test]
fn overlapping_zooms_keep_the_earlier_one() {
    let raw = r#"{"edits":[{"kind":"zoom","at_ms":1000,"dur_ms":2000,"why":"first"},
                           {"kind":"zoom","at_ms":2000,"dur_ms":2000,"why":"overlaps"},
                           {"kind":"zoom","at_ms":6000,"dur_ms":1000,"why":"clear"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(
        p.iter().map(|x| x.at_ms).collect::<Vec<_>>(),
        vec![1_000, 6_000]
    );
}

#[test]
fn a_layout_switch_back_to_the_screen_is_not_worth_proposing() {
    let raw = r#"{"edits":[{"kind":"layout","at_ms":1000,"dur_ms":3000,"layout":"screen","why":"back to the screen"},
                           {"kind":"layout","at_ms":6000,"dur_ms":3000,"layout":"presenter","why":"you speak"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1);
    assert!(matches!(&p[0].ops[0], EditOp::AddLayoutSeg { layout, .. } if layout == "presenter"));
}

#[test]
fn a_spotlight_becomes_a_region_plus_a_radius_sized_from_the_rect() {
    let raw = r#"{"edits":[{"kind":"spotlight","at_ms":2000,"dur_ms":2500,"rect":[0.4,0.4,0.2,0.1],"why":"you point at the toolbar"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert!(matches!(
        &p[0].ops[0],
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 2_000,
            end_ms: 4_500
        }
    ));
    assert!(matches!(&p[0].ops[1],
        EditOp::UpdateEffect { id, radius: Some(r), .. } if id == NEW_ID && (*r - 0.10).abs() < 1e-4));
}

#[test]
fn a_spotlight_with_no_rect_is_dropped() {
    let raw = r#"{"edits":[{"kind":"spotlight","at_ms":2000,"dur_ms":2000,"why":"somewhere"}]}"#;
    assert!(proposals_from_json(raw, CLIP, &[]).is_empty());
}

#[test]
fn a_head_only_trim_keeps_the_zero_out_ms_sentinel() {
    let raw = r#"{"edits":[{"kind":"trim","in_ms":2000,"out_ms":0,"why":"dead air"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert!(matches!(
        &p[0].ops[0],
        EditOp::SetTrim {
            in_ms: 2_000,
            out_ms: 0
        }
    ));
}

#[test]
fn a_zero_zero_trim_is_no_decision_at_all() {
    let raw = r#"{"edits":[{"kind":"trim","in_ms":0,"out_ms":0,"why":"nothing"}]}"#;
    assert!(proposals_from_json(raw, CLIP, &[]).is_empty());
}

#[test]
fn cut_and_speed_map_onto_the_ops_that_now_render_them() {
    let raw = r#"{"edits":[{"kind":"cut","at_ms":4000,"dur_ms":3000,"why":"dead air in the middle"},
                           {"kind":"speed","at_ms":8000,"dur_ms":6000,"factor":2.0,"why":"a long password"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 2, "{:?}", p);
    assert!(
        matches!(&p[0].ops[0], EditOp::AddCuts { spans } if spans == &[(4_000u32, 7_000u32)]),
        "one span, one op, one undo step: {:?}",
        p[0].ops
    );
    assert!(matches!(&p[1].ops[0],
        EditOp::SetSpeed { start_ms: 8_000, end_ms: 14_000, factor } if (*factor - 2.0).abs() < 1e-4));
}

#[test]
fn a_speed_span_with_no_factor_is_not_a_decision_worth_applying() {
    let raw = r#"{"edits":[{"kind":"speed","at_ms":8000,"dur_ms":6000,"why":"faster somehow"}]}"#;
    assert!(proposals_from_json(raw, CLIP, &[]).is_empty());
}

#[test]
fn a_rambling_why_is_trimmed_to_one_line() {
    let long = "word ".repeat(80);
    let raw =
        format!(r#"{{"edits":[{{"kind":"zoom","at_ms":1000,"dur_ms":1000,"why":"{long}"}}]}}"#);
    let p = proposals_from_json(&raw, CLIP, &[]);
    assert!(
        p[0].why.chars().count() <= WHY_MAX,
        "{} chars",
        p[0].why.chars().count()
    );
    assert!(!p[0].why.contains('\n'));
}

#[test]
fn an_unknown_kind_is_dropped_without_taking_the_run_with_it() {
    let raw = r#"{"edits":[{"kind":"add_music","at_ms":1000,"dur_ms":1000,"why":"vibes"},{"kind":"zoom","at_ms":5000,"dur_ms":1000,"why":"real"}]}"#;
    let p = proposals_from_json(raw, CLIP, &[]);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].kind, ProposalKind::Zoom);
}
