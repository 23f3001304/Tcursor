use super::*;
use crate::edit::model::{Arrangement, PanelPose};
use crate::export::scene::arrangement::{arrangement_of_preset, resolve_arrangement};

const OW: u32 = 3840;
const OH: u32 = 2160;

fn with_arr(s: LayoutSeg, a: Arrangement) -> LayoutSeg {
    LayoutSeg {
        arrangement: Some(a),
        ..s
    }
}

fn arr_scene(app: &AppearanceSettings, layout: &str, a: &Arrangement) -> Scene {
    let id = crate::export::render::fromedit::layout_id_from(layout);
    let ma = app.for_id(id);
    let (l, ov) = (layout_for(ma, OW, OH), overlay_for(ma, OW, OH, true));
    resolve_arrangement(a, resolve(id, &l, &ov, 1920, 1080), &l, &ov, 1920, 1080)
}

fn pose(cx: f32, cy: f32, size: f32) -> Option<PanelPose> {
    Some(PanelPose { cx, cy, size })
}

fn custom() -> Arrangement {
    Arrangement {
        screen: pose(0.35, 0.5, 0.6),
        cam: pose(0.8, 0.3, 0.3),
    }
}

#[test]
fn an_arrangement_segment_resolves_from_its_poses_not_its_preset_name() {
    let app = AppearanceSettings::default();
    let a = custom();
    let segs = vec![with_arr(seg("l0", 0, 4000, "screen", 0), a)];
    let track = LayoutTrack::from_segs(&segs, &app, OW, OH, 1920, 1080);
    assert_eq!(track.scene_at(2000), arr_scene(&app, "screen", &a));
    assert_ne!(
        track.scene_at(2000),
        scene_of(&app, LayoutId::Screen),
        "the preset must NOT win"
    );
    let s = track.scene_at(2000).screen.rect;
    assert!((s.x + s.w / 2.0 - 0.35 * OW as f32).abs() < 0.01);
    assert!((s.h - 0.6 * OH as f32).abs() < 0.01);
}

#[test]
fn the_layout_name_still_selects_the_appearance_block() {
    let bubble = crate::settings::appearance::ModeAppearance::default();
    let app = AppearanceSettings {
        screen: bubble,
        camera: crate::settings::appearance::ModeAppearance {
            cam_shape: crate::settings::appearance::CamShape::Rect,
            ..bubble
        },
        ..AppearanceSettings::default()
    };
    let a = custom();
    let by_screen = arr_scene(&app, "screen", &a);
    let by_camera = arr_scene(&app, "camera", &a);
    assert_eq!(
        by_screen.screen.rect, by_camera.screen.rect,
        "the poses are identical"
    );
    assert!(
        by_screen.camera.radius > 0.0 && by_camera.camera.radius == 0.0,
        "but the cam shape comes from the named mode's appearance"
    );
}

#[test]
fn a_hidden_panel_in_a_segment_is_alpha_zero_at_the_preset_rect() {
    let app = AppearanceSettings::default();
    let a = Arrangement {
        screen: pose(0.5, 0.5, 0.9),
        cam: None,
    };
    let segs = vec![with_arr(seg("l0", 0, 4000, "presenter", 0), a)];
    let track = LayoutTrack::from_segs(&segs, &app, OW, OH, 1920, 1080);
    let s = track.scene_at(1000);
    assert_eq!(s.camera.alpha, 0.0);
    assert_eq!(
        s.camera.rect,
        scene_of(&app, LayoutId::Presenter).camera.rect
    );
}

#[test]
fn a_preset_to_arrangement_crossfade_is_the_same_lerp_as_preset_to_preset() {
    let app = AppearanceSettings::default();
    let a = custom();
    let segs = vec![
        seg("l0", 0, 1000, "camera", 0),
        with_arr(seg("l1", 1000, 4000, "screen", 400), a),
    ];
    let track = LayoutTrack::from_segs(&segs, &app, OW, OH, 1920, 1080);
    let (from, to) = (
        scene_of(&app, LayoutId::Camera),
        arr_scene(&app, "screen", &a),
    );
    assert_eq!(
        track.scene_at(1000),
        from,
        "the fade starts AT the outgoing preset"
    );
    assert_eq!(
        track.scene_at(1200),
        Scene::lerp(&from, &to, ease(Easing::Smooth, 0.5))
    );
    assert_eq!(
        track.scene_at(1400),
        to,
        "and lands exactly on the arrangement"
    );
    assert!(
        rect_delta(&track.scene_at(1200), &from) > 1.0
            && rect_delta(&track.scene_at(1200), &to) > 1.0
    );
}

#[test]
fn an_arrangement_to_preset_exit_blend_lands_on_the_successor_at_end_ms() {
    let app = AppearanceSettings::default();
    let a = custom();
    let segs = vec![with_exit(with_arr(seg("l0", 0, 1000, "camera", 0), a), 400)];
    let track = LayoutTrack::from_segs(&segs, &app, OW, OH, 1920, 1080);
    let (from, to) = (
        arr_scene(&app, "camera", &a),
        scene_of(&app, LayoutId::Screen),
    );
    assert_eq!(
        track.scene_at(599),
        from,
        "before the exit window nothing has moved"
    );
    assert_eq!(
        track.scene_at(800),
        Scene::lerp(&from, &to, ease(Easing::Smooth, 0.5))
    );
    assert!(
        rect_delta(&track.scene_at(999), &to) < 0.05,
        "converges on the gap default, no jump"
    );
    assert_eq!(track.scene_at(1000), to);
}

#[test]
fn a_segment_converted_from_its_own_preset_renders_the_same_frame() {
    let app = AppearanceSettings::default();
    for (name, id) in [
        ("screen", LayoutId::Screen),
        ("camera", LayoutId::Camera),
        ("presenter", LayoutId::Presenter),
        ("screen_only", LayoutId::ScreenOnly),
        ("camera_only", LayoutId::CameraOnly),
    ] {
        let preset = scene_of(&app, id);
        let a = arrangement_of_preset(&preset, OW as f32, OH as f32);
        let segs = vec![with_arr(seg("l0", 0, 4000, name, 0), a)];
        let track = LayoutTrack::from_segs(&segs, &app, OW, OH, 1920, 1080);
        let d = rect_delta(&track.scene_at(2000), &preset);
        assert!(
            d < 0.5,
            "{name}: converted segment drifted {d}px from its preset"
        );
        assert_eq!(
            track.scene_at(2000).camera.alpha,
            preset.camera.alpha,
            "{name} cam visibility"
        );
        assert_eq!(
            track.scene_at(2000).screen.alpha,
            preset.screen.alpha,
            "{name} screen visibility"
        );
    }
}
