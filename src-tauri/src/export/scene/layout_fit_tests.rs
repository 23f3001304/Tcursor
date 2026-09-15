use super::*;
use crate::actions::model::LayoutId;
use crate::edit::model::LayoutSeg;
use crate::export::scene::{resolve, Scene};
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

fn scene_of(app: &AppearanceSettings, id: LayoutId) -> Scene {
    let ma = app.for_id(id);
    resolve(
        id,
        &layout_for(ma, 3840, 2160),
        &overlay_for(ma, 3840, 2160, true),
        1920,
        1080,
    )
}

fn seg(id: &str, start: u32, end: u32, layout: &str, transition_ms: u32) -> LayoutSeg {
    LayoutSeg {
        id: id.into(),
        start_ms: start,
        end_ms: end,
        layout: layout.into(),
        transition_ms,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    }
}
fn with_exit(s: LayoutSeg, transition_out_ms: u32) -> LayoutSeg {
    LayoutSeg {
        transition_out_ms,
        ..s
    }
}

fn rect_delta(a: &Scene, b: &Scene) -> f32 {
    let (p, q) = (a.screen.rect, b.screen.rect);
    [
        (p.x - q.x).abs(),
        (p.y - q.y).abs(),
        (p.w - q.w).abs(),
        (p.h - q.h).abs(),
    ]
    .into_iter()
    .fold(0.0f32, f32::max)
}

#[test]
fn a_segments_own_entry_and_exit_are_fitted_into_a_span_too_short_for_both() {
    let app = AppearanceSettings::default();
    let segs = vec![with_exit(seg("l0", 1000, 1300, "camera", 300), 300)];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    let (screen, camera) = (
        scene_of(&app, LayoutId::Screen),
        scene_of(&app, LayoutId::Camera),
    );
    assert_eq!(track.scene_at(1000), screen, "entry at f=0");
    assert!(
        rect_delta(&track.scene_at(1150), &camera) < 1.0,
        "the 150ms entry ARRIVES at the midpoint"
    );
    assert!(
        rect_delta(&track.scene_at(1299), &screen) < rect_delta(&track.scene_at(1150), &screen),
        "and the 150ms exit carries it back toward the base before end_ms"
    );
}

#[test]
fn a_segment_shorter_than_its_transitions_hands_over_without_a_jump() {
    let app = AppearanceSettings::default();
    let segs = vec![
        seg("a", 0, 1000, "screen", 0),
        seg("b", 1000, 1200, "camera", 350),
        seg("c", 1200, 3000, "presenter", 350),
    ];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    let step = rect_delta(&track.scene_at(1180), &track.scene_at(1199));
    let boundary = rect_delta(&track.scene_at(1199), &track.scene_at(1200));
    assert!(
        boundary <= step.max(1.0) * 2.0,
        "boundary jump {boundary}px dwarfs an ordinary {step}px step - the segment never settled"
    );
    assert!(
        rect_delta(&track.scene_at(1199), &scene_of(&app, LayoutId::Camera)) < 1.0,
        "the short segment never arrived at its own scene"
    );
}

#[test]
fn an_entry_and_exit_that_would_overlap_are_fitted_instead() {
    let app = AppearanceSettings::default();
    let segs = vec![
        seg("a", 0, 1000, "screen", 0),
        with_exit(seg("b", 1000, 1400, "camera", 350), 350),
        seg("c", 1400, 3000, "presenter", 0),
    ];
    let track = LayoutTrack::from_segs(&segs, &app, 3840, 2160, 1920, 1080);
    let worst = (1000..1400)
        .map(|t| rect_delta(&track.scene_at(t), &track.scene_at(t + 1)))
        .fold(0.0f32, f32::max);
    assert!(
        worst < 40.0,
        "a single 1ms step moved the panel {worst}px - entry and exit overlapped"
    );
}
