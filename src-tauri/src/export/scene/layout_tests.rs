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
    LayoutSeg { id: id.into(), start_ms: start, end_ms: end, layout: layout.into(), transition_ms, easing: "smooth".into(),
        transition_out_ms: 0, easing_out: "smooth".into(), arrangement: None }
}
fn with_exit(s: LayoutSeg, transition_out_ms: u32) -> LayoutSeg { LayoutSeg { transition_out_ms, ..s } }
/// Largest absolute difference between two scenes' screen-panel rects - the continuity yardstick.
fn rect_delta(a: &Scene, b: &Scene) -> f32 {
    let (p, q) = (a.screen.rect, b.screen.rect);
    [(p.x - q.x).abs(), (p.y - q.y).abs(), (p.w - q.w).abs(), (p.h - q.h).abs()]
        .into_iter().fold(0.0f32, f32::max)
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

#[test]
fn default_zero_exit_is_the_historical_hard_cut() {
    let app = AppearanceSettings::default();
    // The exact fixture from `a_gap_between_segments_falls_back_to_screen`, which predates exit
    // transitions: with `transition_out_ms` defaulting to 0 every sample must be bit-identical.
    let segs = vec![seg("l0", 0, 1000, "camera", 0), seg("l1", 2000, 3000, "presenter", 0)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    assert_eq!(track.scene_at(999), scene_of(&app, LayoutId::Camera), "last active ms is still fully camera");
    assert_eq!(track.scene_at(1000), scene_of(&app, LayoutId::Screen), "and it cuts to the gap default");
}

#[test]
fn exit_blend_completes_exactly_at_end_ms() {
    let app = AppearanceSettings::default();
    let (camera, screen) = (scene_of(&app, LayoutId::Camera), scene_of(&app, LayoutId::Screen));
    // camera [0,1000) with a 400ms exit, then a GAP -> it must ease back to the screen default.
    let segs = vec![with_exit(seg("l0", 0, 1000, "camera", 0), 400)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    assert_eq!(track.scene_at(599), camera, "before the exit window nothing has moved");
    assert_eq!(track.scene_at(600), camera, "the window opens AT f=0, i.e. still fully camera");
    let mid = track.scene_at(800);
    assert!(rect_delta(&mid, &camera) > 1.0 && rect_delta(&mid, &screen) > 1.0, "mid-exit is between the two");
    // The pose the exit converges on IS what the track resolves at end_ms, to within a hair.
    assert!(rect_delta(&track.scene_at(999), &track.scene_at(1000)) < 0.05,
        "the exit must land on the successor's pose, not jump to it");
    assert_eq!(track.scene_at(1000), screen);
    // The blend fraction itself reaches exactly 1 at end_ms, so the convergence above is exact in
    // the limit and only ms quantisation separates the last active sample from the successor.
    assert_eq!(ease(Easing::Smooth, (1000u32 - 600) as f32 / 400.0), 1.0);
    assert_eq!(Scene::lerp(&camera, &screen, 1.0), screen);
}

#[test]
fn a_gapless_successors_entry_wins_the_overlap() {
    let app = AppearanceSettings::default();
    let camera = scene_of(&app, LayoutId::Camera);
    // camera [0,1000) with a 400ms exit, presenter [1000,2000) with a 400ms ENTRY. Only one blend
    // may run: the entry wins, so the exit stands down and camera holds right up to 1000 - exactly
    // as it did before exit transitions existed (no double-blend, no backwards jump at 1000).
    let segs = vec![with_exit(seg("l0", 0, 1000, "camera", 0), 400), seg("l1", 1000, 2000, "presenter", 400)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    for t in [600, 800, 999] { assert_eq!(track.scene_at(t), camera, "exit suppressed at {t}"); }
    assert_eq!(track.scene_at(1000), camera, "the successor's entry starts FROM camera - seamless");
    assert_eq!(track.scene_at(1400), scene_of(&app, LayoutId::Presenter));
    // ...but a successor that hard-cuts in (no entry transition of its own) does NOT win, so the
    // exit runs and smooths a switch that used to be an instant pop.
    let hard = vec![with_exit(seg("l0", 0, 1000, "camera", 0), 400), seg("l1", 1000, 2000, "presenter", 0)];
    let track = LayoutTrack::from_segs(&hard, &app, 3840, 2160, 1920, 1080);
    assert_ne!(track.scene_at(800), camera, "exit runs into a hard-cutting successor");
    assert!(rect_delta(&track.scene_at(999), &scene_of(&app, LayoutId::Presenter)) < 0.05);
}

// Arrangement (pose-driven) segment tests, likewise in their own file for the size budget.
#[path = "layout_arrangement_tests.rs"]
mod arrangement_tests;

#[test]
fn a_segments_own_entry_and_exit_are_fitted_into_a_span_too_short_for_both() {
    let app = AppearanceSettings::default();
    // A 300ms segment carrying a 300ms entry AND a 300ms exit. The two used to cover each other
    // completely and entry simply outranked exit, so the segment ramped in across its WHOLE span,
    // never arrived, and hard-cut back at `end_ms`. Fitted, it is a symmetric bump: 150ms in,
    // arriving at the midpoint, then 150ms back out.
    // (It starts at 1000, not 0: `raw_scene(start - 1)` saturates for a segment at t=0, which
    // makes its own entry a no-op - a pre-existing quirk this test must not depend on.)
    let segs = vec![with_exit(seg("l0", 1000, 1300, "camera", 300), 300)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    let (screen, camera) = (scene_of(&app, LayoutId::Screen), scene_of(&app, LayoutId::Camera));
    assert_eq!(track.scene_at(1000), screen, "entry at f=0");
    assert!(rect_delta(&track.scene_at(1150), &camera) < 1.0, "the 150ms entry ARRIVES at the midpoint");
    assert!(rect_delta(&track.scene_at(1299), &screen) < rect_delta(&track.scene_at(1150), &screen),
        "and the 150ms exit carries it back toward the base before end_ms");
}

/// A segment whose own transitions are longer than the segment itself used to leave a HOLE in the
/// animation: the entry never finished, so the segment never reached its own scene - but the NEXT
/// segment's entry blends from `raw_scene`, which hands it that unreached full scene anyway. The
/// result was a one-frame jump at the boundary (measured at ~0.16 frame-widths on a 200ms segment
/// with a 350ms entry). Transitions are now fitted into the span, exactly as `fit_durations` keeps
/// a zoom's ramps inside its pill, so the segment settles before it hands over.
#[test]
fn a_segment_shorter_than_its_transitions_hands_over_without_a_jump() {
    let app = AppearanceSettings::default();
    let segs = vec![
        seg("a", 0, 1000, "screen", 0),
        seg("b", 1000, 1200, "camera", 350),     // 350ms entry inside a 200ms segment
        seg("c", 1200, 3000, "presenter", 350),
    ];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    // The frame before the boundary and the frame at it must be continuous: the gap across 1199 ->
    // 1200 has to be the size of one ordinary animation step, not a snap to an unreached scene.
    let step = rect_delta(&track.scene_at(1180), &track.scene_at(1199));
    let boundary = rect_delta(&track.scene_at(1199), &track.scene_at(1200));
    assert!(boundary <= step.max(1.0) * 2.0,
        "boundary jump {boundary}px dwarfs an ordinary {step}px step - the segment never settled");
    // And it settles: a fitted entry lands exactly AT `end_ms` (the same contract the exit blend
    // has), so by its last frame the short segment is showing its own scene to within a pixel -
    // which is what makes the hand-over above continuous.
    assert!(rect_delta(&track.scene_at(1199), &scene_of(&app, LayoutId::Camera)) < 1.0,
        "the short segment never arrived at its own scene");
}

/// The same fitting keeps an entry and an exit from OVERLAPPING inside a short segment. Unfitted,
/// a 400ms segment with 350ms on each side ran its exit from t=50 while the entry still owned the
/// frame to t=350, so the instant the entry expired the scene lurched most of the way to the
/// successor in a single frame.
#[test]
fn an_entry_and_exit_that_would_overlap_are_fitted_instead() {
    let app = AppearanceSettings::default();
    let segs = vec![
        seg("a", 0, 1000, "screen", 0),
        with_exit(seg("b", 1000, 1400, "camera", 350), 350), // 700ms of transition in 400ms
        seg("c", 1400, 3000, "presenter", 0),
    ];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    let worst = (1000..1400).map(|t| rect_delta(&track.scene_at(t), &track.scene_at(t + 1)))
        .fold(0.0f32, f32::max);
    assert!(worst < 40.0, "a single 1ms step moved the panel {worst}px - entry and exit overlapped");
}
