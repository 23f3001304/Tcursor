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

#[path = "fx_state_spot_tests.rs"]
mod spot_tests;
