// Tests for export::scene::layout, split out so layout.rs stays under the size limit.
use super::*;
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::LayoutSeg;
use crate::export::scene::{resolve, Scene};
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

fn scene_of(app: &AppearanceSettings, id: LayoutId) -> Scene {
    let ma = app.for_id(id);
    resolve(id, &layout_for(ma, 3840, 2160), &overlay_for(ma, 3840, 2160, true), 1920, 1080)
}

fn seg(id: &str, start: u32, end: u32, layout: &str, transition_ms: u32) -> LayoutSeg {
    LayoutSeg { id: id.into(), start_ms: start, end_ms: end, layout: layout.into(), transition_ms, easing: "smooth".into() }
}

#[test]
fn no_actions_is_screenfocus_everywhere() {
    let app = AppearanceSettings::default();
    let track = LayoutTrack::new(&[], &app, 3840, 2160, 1920, 1080, 350);
    let screen = scene_of(&app, LayoutId::Screen);
    assert_eq!(track.scene_at(0), screen);
    assert_eq!(track.scene_at(100_000), screen);
}

#[test]
fn switch_transitions_then_settles() {
    let app = AppearanceSettings::default();
    let acts = vec![ActionEvent { t: 1000, kind: ActionKind::SetLayout(LayoutId::Camera) }];
    let track = LayoutTrack::new(&acts, &app, 3840, 2160, 1920, 1080, 400);
    let screen = scene_of(&app, LayoutId::Screen);
    let camera = scene_of(&app, LayoutId::Camera);
    assert_eq!(track.scene_at(999), screen);
    assert_eq!(track.scene_at(1000), screen); // fade just started (t=0 of the ease)
    assert_eq!(track.scene_at(1400), camera);
    let mid = track.scene_at(1200);
    let (lo, hi) = (camera.screen.rect.w.min(screen.screen.rect.w), camera.screen.rect.w.max(screen.screen.rect.w));
    assert!(mid.screen.rect.w > lo && mid.screen.rect.w < hi);
}

#[test]
fn latest_switch_wins() {
    let app = AppearanceSettings::default();
    let acts = vec![
        ActionEvent { t: 100, kind: ActionKind::SetLayout(LayoutId::Camera) },
        ActionEvent { t: 200, kind: ActionKind::SetLayout(LayoutId::Presenter) },
    ];
    let track = LayoutTrack::new(&acts, &app, 3840, 2160, 1920, 1080, 0);
    assert_eq!(track.scene_at(10_000), scene_of(&app, LayoutId::Presenter));
}

#[test]
fn per_segment_transition_feel_drives_the_crossfade() {
    let app = AppearanceSettings::default();
    // seg A screen [0,1000] (snap), seg B camera [1000,4000] with a 400ms fade; at t=1200 (200ms
    // in) we are mid-fade -> the scene must differ from the pure camera scene.
    let segs = vec![seg("l0", 0, 1000, "screen", 0), seg("l1", 1000, 4000, "camera", 400)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    assert_ne!(track.scene_at(1200), scene_of(&app, LayoutId::Camera), "at 200/400ms the fade should not have reached camera yet");
    // transition_ms 0 snaps: at the exact switch it is already the camera scene.
    let snap = vec![seg("l0", 0, 1000, "screen", 0), seg("l1", 1000, 4000, "camera", 0)];
    assert_eq!(LayoutTrack::from_segs(&snap, &app, 3840, 2160, 1920, 1080).scene_at(1000), scene_of(&app, LayoutId::Camera));
}

#[test]
fn a_gap_between_segments_falls_back_to_screen() {
    let app = AppearanceSettings::default();
    let screen = scene_of(&app, LayoutId::Screen);
    // camera [0,1000], GAP [1000,2000], presenter [2000,3000] - the "empty means default" model.
    let segs = vec![seg("l0", 0, 1000, "camera", 0), seg("l1", 2000, 3000, "presenter", 0)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    assert_eq!(track.scene_at(500), scene_of(&app, LayoutId::Camera)); // inside camera
    assert_eq!(track.scene_at(1500), screen);   // GAP -> screen default
    assert_eq!(track.scene_at(3500), screen);   // after the last seg -> screen default
    assert_eq!(track.scene_at(2500), scene_of(&app, LayoutId::Presenter)); // inside presenter
}
