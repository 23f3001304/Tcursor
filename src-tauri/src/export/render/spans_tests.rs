use super::*;
use crate::edit::model::Trim;
use crate::export::remap::TimeMap;

const CANVAS: (u32, u32) = (1920, 1080);

fn plain_map() -> TimeMap {
    TimeMap::build(&Trim::default(), &[], &[], &[], 60_000)
}
fn sw(at_ms: u64, w: u32, h: u32) -> DisplaySwitch {
    DisplaySwitch {
        at_ms,
        target_id: "display:2".into(),
        w,
        h,
    }
}

#[test]
fn no_switch_is_one_full_canvas_span() {
    let s = spans_of(&[], &[], CANVAS, 0, &plain_map());
    assert_eq!(s.len(), 1);
    assert_eq!(
        s[0],
        SourceSpan {
            start_ms: 0,
            src: crate::export::coordmap::full_src(1920, 1080)
        }
    );
}

#[test]
fn a_switch_spans_the_capture_s_own_fitted_rect() {
    let s = spans_of(&[sw(3000, 1280, 800)], &[], CANVAS, 0, &plain_map());
    assert_eq!(s.len(), 2);
    assert_eq!(s[1].start_ms, 3000);
    assert_eq!(
        (s[1].src.x, s[1].src.y, s[1].src.w, s[1].src.h),
        (96.0, 0.0, 1728.0, 1080.0)
    );
}

#[test]
fn the_instant_goes_through_video_start_and_the_time_map() {
    let map = TimeMap::build(
        &Trim {
            in_ms: 1000,
            out_ms: 0,
        },
        &[],
        &[],
        &[],
        60_000,
    );
    let s = spans_of(&[sw(9_500, 1280, 800)], &[], CANVAS, 5_000, &map);
    assert_eq!(s[1].start_ms, 3500);
}

#[test]
fn an_unknown_size_spans_the_full_canvas() {
    let s = spans_of(&[sw(3000, 0, 0)], &[], CANVAS, 0, &plain_map());
    assert_eq!(s[1].src, crate::export::coordmap::full_src(1920, 1080));
}

#[test]
fn switches_on_the_same_instant_collapse_to_the_last() {
    let s = spans_of(
        &[sw(3000, 1280, 800), sw(3000, 1920, 1200)],
        &[],
        CANVAS,
        0,
        &plain_map(),
    );
    assert_eq!(s.len(), 2);
    assert_eq!((s[1].start_ms, s[1].src.w), (3000, 1728.0));
    let s = spans_of(&[sw(0, 1280, 800)], &[], CANVAS, 0, &plain_map());
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].src.w, 1728.0);
}

#[test]
fn a_span_opens_on_the_first_frame_after_the_stamp() {
    let s = spans_of(
        &[sw(3000, 1280, 800)],
        &[2950, 2967, 2983, 3120, 3137],
        CANVAS,
        0,
        &plain_map(),
    );
    assert_eq!(s[1].start_ms, 3120);
    let s = spans_of(
        &[sw(3000, 1280, 800)],
        &[2950, 2983],
        CANVAS,
        0,
        &plain_map(),
    );
    assert_eq!(s[1].start_ms, 3000);
}

#[test]
fn hold_tick_is_the_last_output_frame_before_the_switch() {
    for fps in [30u64, 60] {
        for start in [1u32, 100, 3000, 3001, 16_667] {
            let t = hold_tick(start, fps);
            let j = (0..)
                .find(|j| (j * 1000 / fps) as u32 == t)
                .expect("on the frame grid");
            assert!(
                t < start,
                "fps {fps} start {start}: {t} must precede the switch"
            );
            assert!(
                ((j + 1) * 1000 / fps) as u32 >= start,
                "fps {fps} start {start}: {t} is not the LAST one before it"
            );
        }
    }
    assert_eq!(hold_tick(3000, 60), 2983);
}

fn track(spans: Vec<SourceSpan>) -> SpanTrack {
    let app = crate::settings::appearance::AppearanceSettings::default();
    SpanTrack::build(spans, |w, h| {
        LayoutTrack::new(&[], &app, 1920, 1080, w, h, 350)
    })
}

#[test]
fn a_single_span_never_mixes_and_never_holds() {
    let t = track(spans_of(&[], &[], CANVAS, 0, &plain_map()));
    for ms in [0u32, 500, 3000, 9999] {
        let (scene, mix, hold) = t.frame_at(ms, 60);
        assert_eq!(scene.src, crate::export::coordmap::full_src(1920, 1080));
        assert!(mix.is_none() && hold.is_none(), "at {ms}");
    }
}

#[test]
fn a_switch_eases_the_panel_and_dissolves_the_picture() {
    let t = track(spans_of(
        &[sw(3000, 1280, 800)],
        &[],
        CANVAS,
        0,
        &plain_map(),
    ));
    assert_eq!(
        t.frame_at(hold_tick(3000, 60), 60).2,
        Some(0),
        "the frame before the switch latches"
    );

    let (before, _, _) = t.frame_at(2900, 60);
    let (after, mix_after, _) = t.frame_at(3000 + SWITCH_MS, 60);
    assert!(mix_after.is_none(), "the transition is over by then");
    assert!(
        after.screen.rect.w < before.screen.rect.w - 1.0,
        "16:10 is a narrower panel than 16:9"
    );
    assert_eq!(
        after.screen.rect.h, before.screen.rect.h,
        "both fits are height-limited here"
    );

    let mut last_alpha = -1.0;
    let mut last_w = f32::MAX;
    for ms in [3000, 3050, 3175, 3300, 3349] {
        let (scene, mix, _) = t.frame_at(ms, 60);
        let m = mix.expect("inside the transition");
        assert_eq!(m.prev_span, 0);
        assert_eq!(m.prev_src, crate::export::coordmap::full_src(1920, 1080));
        assert_eq!(m.hold_ms, hold_tick(3000, 60));
        assert!(m.alpha > last_alpha, "alpha must rise: {ms}");
        assert!(
            scene.src.w == 1728.0,
            "content is already the new span's rect"
        );
        assert!(
            scene.screen.rect.w < last_w + 0.001,
            "the panel must narrow monotonically: {ms}"
        );
        (last_alpha, last_w) = (m.alpha, scene.screen.rect.w);
    }
    assert!(
        t.frame_at(3000, 60).1.unwrap().alpha < 0.02,
        "starts on the old picture"
    );
    assert!(
        t.frame_at(3349, 60).1.unwrap().alpha > 0.98,
        "ends on the new one"
    );
}

#[test]
fn scene_at_matches_frame_at() {
    let t = track(spans_of(
        &[sw(3000, 1280, 800)],
        &[],
        CANVAS,
        0,
        &plain_map(),
    ));
    for ms in [0u32, 2999, 3100, 5000] {
        assert_eq!(t.scene_at(ms), t.frame_at(ms, 60).0, "at {ms}");
    }
}
