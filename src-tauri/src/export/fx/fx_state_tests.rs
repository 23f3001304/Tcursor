use super::*;
use crate::edit::model::EffectKind;
use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, FramePoint, RectF};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, SpotlightMode};

fn scr() -> ScreenInfo {
    ScreenInfo {
        w: 100,
        h: 100,
        origin_x: 0,
        origin_y: 0,
    }
}

fn full_scene(w: u32, h: u32) -> Scene {
    Scene {
        screen: Panel {
            rect: RectF {
                x: 0.0,
                y: 0.0,
                w: w as f32,
                h: h as f32,
            },
            radius: 0.0,
            alpha: 1.0,
            ring_px: 0.0,
            ring_color: [0, 0, 0],
        },
        camera: Panel {
            rect: RectF {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            radius: 0.0,
            alpha: 0.0,
            ring_px: 0.0,
            ring_color: [0, 0, 0],
        },
        src: crate::export::coordmap::full_src(w, h),
    }
}
fn fx(style: ClickFxStyle, spot: bool) -> ClickFxSettings {
    ClickFxSettings {
        enabled: true,
        style,
        color: [255, 0, 0],
        intensity: 1.0,
        captions: false,
        spotlight: spot,
        spotlight_dim: 0.6,
        spotlight_radius: 0.13,
        spotlight_feather: 0.10,
        spotlight_mode: SpotlightMode::Classic,
        spotlight_tint: [130, 90, 255],
        video_fx_mode: crate::settings::model::VideoFxMode::NebulaWash,
        spotlight_dim_camera: true,
    }
}
fn cam() -> Camera {
    Camera {
        cx: 50.0,
        cy: 50.0,
        scale: 1.0,
    }
}
fn down(t: u32) -> MouseEvent {
    MouseEvent {
        t,
        kind: EventKind::Down,
        x: 50,
        y: 50,
        button: Some(Button::Left),
    }
}

#[test]
fn nothing_active_is_none() {
    assert!(fx_state_at(
        &fx(ClickFxStyle::Ripple, false),
        &[],
        &[],
        &[],
        &full_scene(100, 100),
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new()
    )
    .is_none());
}

#[test]
fn a_click_makes_a_hit_in_output_space() {
    let s = fx_state_at(
        &fx(ClickFxStyle::Ripple, false),
        &[down(0)],
        &[],
        &[],
        &full_scene(100, 100),
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        300,
        300,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    assert_eq!(s.hits.len(), 1);
    assert!((s.hits[0].x - 50.0).abs() < 1.0 && s.hits[0].progress > 0.0);
}
#[test]
fn style_none_suppresses_click_hits() {
    let s = fx_state_at(
        &fx(ClickFxStyle::None, true),
        &[down(0)],
        &[],
        &[],
        &full_scene(100, 100),
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        100,
        100,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    assert!(s.hits.is_empty() && s.spot.is_some());
}

#[test]
fn click_hit_origin_is_converted_before_panel_mapping() {
    let screen = ScreenInfo {
        w: 1200,
        h: 800,
        origin_x: 500,
        origin_y: 300,
    };
    let down_at_origin = MouseEvent {
        t: 0,
        kind: EventKind::Down,
        x: 500,
        y: 300,
        button: Some(Button::Left),
    };
    let full = full_scene(1200, 800);
    let s = fx_state_at(
        &fx(ClickFxStyle::Ripple, false),
        &[down_at_origin],
        &[],
        &[],
        &full,
        cam(),
        FramePoint { x: 600, y: 400 },
        &screen,
        true,
        1200,
        800,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    assert_eq!(s.hits.len(), 1);
    assert!(s.hits[0].x < 5.0 && s.hits[0].y < 5.0,
        "expected the hit at the panel's top-left corner (origin-converted), got ({}, {}) - looks like the raw un-converted 41.6%/37.5% position", s.hits[0].x, s.hits[0].y);
}

#[test]
fn region_and_event_clocks_are_sampled_independently() {
    let e = EffectRegion {
        id: "e0".into(),
        kind: EffectKind::Spotlight,
        start_ms: 1000,
        end_ms: 2000,
        fade_in_ms: 100,
        fade_out_ms: 100,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
        rect: None,
        strength: None,
        roundness: None,
    };
    let s = fx_state_at(
        &fx(ClickFxStyle::Ripple, false),
        &[down(2000)],
        &[],
        &[e],
        &full_scene(100, 100),
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        1500,
        2300,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s
        .spot
        .expect("the region must be sampled at region_t (1500), not ev_t (2300)");
    assert_eq!(
        s.hits.len(),
        1,
        "the click must be sampled at ev_t (2300), not region_t (1500)"
    );
    assert!(
        (spot.t - 1.5).abs() < 1e-6,
        "spot.t should be region_t seconds: {}",
        spot.t
    );
}

#[test]
fn a_mask_survives_click_animations_being_switched_off() {
    use crate::export::fx::fx_masks::MaskDraw;
    let (w, h) = (80u32, 60u32);
    let mut out = vec![200u8; (w * h * 4) as usize];
    let mut fx = crate::settings::model::ClickFxSettings::default();
    fx.enabled = false;
    fx.spotlight = false;
    let masks = vec![MaskDraw {
        mn: [10.0, 10.0],
        mx: [40.0, 40.0],
        r: 0.0,
        feather_px: 1.0,
        amount_px: 4.0,
        dim: 0.6,
        kind: 3,
        alpha: 1.0,
    }];
    crate::export::fx::fx_state::render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &fx,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        crate::export::types::FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut crate::export::fx::fx_state::SpotlightSim::new(),
        None,
        masks,
        None,
    );
    let inside = out[((25 * w + 25) * 4) as usize];
    let outside = out[((55 * w + 5) * 4) as usize];
    assert_eq!(inside, 200, "the highlighted rect keeps its brightness");
    assert!(outside < 120, "everything outside it is dimmed: {outside}");
}

#[path = "fx_state_spot_tests.rs"]
mod spot_tests;

#[test]
fn a_grade_runs_with_click_animations_off_and_no_spotlight() {
    use crate::export::grade::params_of;
    use crate::settings::grade::{GradePreset, GradeSettings};
    let (w, h) = (32u32, 32u32);
    let mut out = vec![180u8; (w * h * 4) as usize];
    let mut settings = fx(ClickFxStyle::Ripple, false);
    settings.enabled = false;
    let (exposure, contrast, vignette) = crate::export::grade::seed_of(GradePreset::Noir);
    let g = params_of(&GradeSettings {
        preset: GradePreset::Noir,
        exposure,
        contrast,
        vignette,
    });
    render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &settings,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut SpotlightSim::new(),
        None,
        Vec::new(),
        g,
    );
    let mid = ((16 * w + 16) * 4) as usize;
    assert_ne!(
        out[mid], 180,
        "the grade ran with every click effect switched off"
    );
    assert!(out[0] < out[mid], "and its vignette darkened the corner");
}

#[test]
fn no_grade_and_nothing_else_leaves_the_frame_alone() {
    let (w, h) = (16u32, 16u32);
    let mut out = vec![77u8; (w * h * 4) as usize];
    let mut settings = fx(ClickFxStyle::Ripple, false);
    settings.enabled = false;
    render(
        &crate::export::fx::fxdraw::CpuFx,
        &mut out,
        w,
        h,
        &settings,
        &[],
        &[],
        &[],
        &full_scene(w, h),
        cam(),
        FramePoint { x: 0, y: 0 },
        &scr(),
        false,
        0,
        0,
        &crate::settings::model::HotkeySettings::default(),
        &mut SpotlightSim::new(),
        None,
        Vec::new(),
        None,
    );
    assert!(
        out.iter().all(|&b| b == 77),
        "an ungraded project with no effects costs nothing"
    );
}
