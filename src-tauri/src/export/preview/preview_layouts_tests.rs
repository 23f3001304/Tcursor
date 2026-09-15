use super::*;
use crate::edit::model::{Arrangement, LayoutSeg, PanelPose};
use crate::export::scene::layout::resolve_seg_scene;
use crate::settings::appearance::AppearanceSettings;

const OW: u32 = 1920;
const OH: u32 = 1080;
const SW: u32 = 1920;
const SH: u32 = 1080;

fn seg(id: &str, layout: &str, arrangement: Option<Arrangement>) -> LayoutSeg {
    LayoutSeg {
        id: id.into(),
        start_ms: 0,
        end_ms: 1000,
        layout: layout.into(),
        transition_ms: 0,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement,
    }
}

#[test]
fn a_plain_preset_segment_gets_a_null_entry_and_never_resolves_a_scene() {
    let s = seg("l0", "camera", None);
    let dto = seg_rect_dto(
        &s,
        || panic!("must not resolve a scene for a plain preset segment"),
        OW as f32,
        OH as f32,
    );
    assert_eq!(dto.id, "l0");
    assert!(dto.screen.is_none() && dto.cam.is_none());
}

#[test]
fn an_arrangement_segment_resolves_both_panels_through_the_real_pose_path() {
    let app = AppearanceSettings::default();
    let a = Arrangement {
        screen: Some(PanelPose {
            cx: 0.35,
            cy: 0.5,
            size: 0.6,
        }),
        cam: Some(PanelPose {
            cx: 0.8,
            cy: 0.3,
            size: 0.3,
        }),
    };
    let s = seg("l0", "screen", Some(a));
    let expected = resolve_seg_scene(&s, &app, OW, OH, SW, SH);
    let dto = seg_rect_dto(
        &s,
        || resolve_seg_scene(&s, &app, OW, OH, SW, SH),
        OW as f32,
        OH as f32,
    );

    let screen = dto.screen.expect("posed screen must resolve to Some");
    let cam = dto.cam.expect("posed cam must resolve to Some");
    assert_eq!(
        screen.rect,
        panel_dto(&expected.screen, OW as f32, OH as f32).rect
    );
    assert_eq!(
        cam.rect,
        panel_dto(&expected.camera, OW as f32, OH as f32).rect
    );
    assert!(
        (screen.rect[0] + screen.rect[2] / 2.0 - 0.35).abs() < 0.01,
        "screen center should sit at the posed cx"
    );
    assert!(
        (screen.rect[3] - 0.6).abs() < 0.01,
        "screen height should be the posed size fraction"
    );
}

#[test]
fn a_hidden_posed_panel_still_resolves_to_some_at_alpha_zero() {
    let app = AppearanceSettings::default();
    let a = Arrangement {
        screen: Some(PanelPose {
            cx: 0.5,
            cy: 0.5,
            size: 0.9,
        }),
        cam: None,
    };
    let s = seg("l0", "presenter", Some(a));
    let dto = seg_rect_dto(
        &s,
        || resolve_seg_scene(&s, &app, OW, OH, SW, SH),
        OW as f32,
        OH as f32,
    );
    let cam = dto
        .cam
        .expect("a hidden posed panel still gets an entry, not a bare None");
    assert_eq!(cam.alpha, 0.0);
}

#[test]
fn the_dto_id_matches_the_segment_id() {
    let with = seg(
        "posed-seg",
        "camera",
        Some(Arrangement {
            screen: None,
            cam: Some(PanelPose {
                cx: 0.5,
                cy: 0.5,
                size: 0.3,
            }),
        }),
    );
    let app = AppearanceSettings::default();
    assert_eq!(
        seg_rect_dto(
            &with,
            || resolve_seg_scene(&with, &app, OW, OH, SW, SH),
            OW as f32,
            OH as f32
        )
        .id,
        "posed-seg"
    );
    let without = seg("preset-seg", "camera", None);
    assert_eq!(
        seg_rect_dto(&without, || unreachable!(), OW as f32, OH as f32).id,
        "preset-seg"
    );
}
