// Tests for export::fx::fx_state, split into their own file so fx_state.rs stays under the size limit.
use super::*;
use crate::edit::model::EffectKind;
use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, FramePoint, RectF};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, SpotlightMode};

/// Origin-less `ScreenInfo` (single-monitor-at-0,0 capture) - a no-op for `to_frame`, so every
/// pre-existing test below keeps its exact original coordinates/assertions.
fn scr() -> ScreenInfo { ScreenInfo { w: 100, h: 100, origin_x: 0, origin_y: 0 } }

fn full_scene(w: u32, h: u32) -> Scene {
    Scene {
        screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: w as f32, h: h as f32 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        src: crate::export::coordmap::full_src(w, h),
    }
}
fn fx(style: ClickFxStyle, spot: bool) -> ClickFxSettings {
    ClickFxSettings { enabled: true, style, color: [255, 0, 0], intensity: 1.0, captions: false,
        spotlight: spot, spotlight_dim: 0.6, spotlight_radius: 0.13, spotlight_feather: 0.10,
        spotlight_mode: SpotlightMode::Classic, spotlight_tint: [130, 90, 255],
        video_fx_mode: crate::settings::model::VideoFxMode::NebulaWash, spotlight_dim_camera: true }
}
fn cam() -> Camera { Camera { cx: 50.0, cy: 50.0, scale: 1.0 } }
fn down(t: u32) -> MouseEvent { MouseEvent { t, kind: EventKind::Down, x: 50, y: 50, button: Some(Button::Left) } }

#[test]
fn nothing_active_is_none() {
    assert!(fx_state_at(&fx(ClickFxStyle::Ripple, false), &[], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 0, 0, &mut SpotlightSim::new()).is_none());
}
#[test]
fn spotlight_toggle_makes_a_spot_at_cursor() {
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 0, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    assert!((spot.alpha - 1.0).abs() < 1e-6);
    assert!((spot.cx - 50.0).abs() < 1.0 && (spot.cy - 50.0).abs() < 1.0);
    assert!(s.hits.is_empty());
    // Full-frame screen panel -> radius unchanged (fraction of the whole frame == of the panel).
    assert!((spot.radius_frac - 0.13).abs() < 1e-6);
}

#[test]
fn spotlight_radius_scales_with_screen_panel_height() {
    // A layout where the screen panel is half the output height must halve the spotlight radius,
    // vs. the old layout-agnostic behaviour (a fixed fraction of the full frame regardless of
    // where/how big the screen actually is).
    let half = Scene {
        screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        src: crate::export::coordmap::full_src(100, 100),
    };
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[], &[], &[], &half, cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 0, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    // screen.h / oh = 50/100 = 0.5, so radius/feather are half their settings-space value.
    assert!((spot.radius_frac - 0.13 * 0.5).abs() < 1e-6, "radius_frac should scale by screen height fraction: {}", spot.radius_frac);
    assert!((spot.feather_frac - 0.10 * 0.5).abs() < 1e-6, "feather_frac should scale too: {}", spot.feather_frac);
}
/// The spotlight "keep camera lit" hole must never apply without a REAL, VISIBLE webcam -
/// otherwise an un-dimmed empty rectangle appears (ScreenOnly layouts, post-Hide, or recordings
/// with no webcam.mp4 at all). `dim_camera: true` is the "no hole" representation - it forces
/// `draw_spot`'s un-dim branch to never run, regardless of the user's actual setting.
#[test]
fn spotlight_hole_disabled_without_a_real_webcam() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false; // user wants the hole ON
    let mut scene = full_scene(100, 100);
    scene.camera.alpha = 1.0; // panel WOULD be visible, but there is no webcam.mp4
    let s = fx_state_at(&settings, &[], &[], &[], &scene, cam(),
        FramePoint{x:50,y:50}, &scr(), false, 100,100, 0, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    assert!(spot.dim_camera, "no webcam.mp4 -> the hole must not apply, regardless of the user's setting");
}

#[test]
fn spotlight_hole_disabled_when_the_camera_panel_is_invisible() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false;
    let scene = full_scene(100, 100); // camera.alpha == 0.0 (ScreenOnly / post-Hide default)
    let s = fx_state_at(&settings, &[], &[], &[], &scene, cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 0, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    assert!(spot.dim_camera, "camera.alpha <= 0.05 -> the hole must not apply even with a real webcam");
}

#[test]
fn spotlight_hole_enabled_with_a_real_visible_webcam() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false;
    let mut scene = full_scene(100, 100);
    scene.camera.alpha = 1.0;
    let s = fx_state_at(&settings, &[], &[], &[], &scene, cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 0, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    assert!(!spot.dim_camera, "a real, visible webcam + dim_camera:false -> the hole DOES apply");
}

#[test]
fn a_click_makes_a_hit_in_output_space() {
    let s = fx_state_at(&fx(ClickFxStyle::Ripple, false), &[down(0)], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 300, 300, &mut SpotlightSim::new()).unwrap();
    assert_eq!(s.hits.len(), 1);
    assert!((s.hits[0].x - 50.0).abs() < 1.0 && s.hits[0].progress > 0.0);
}
#[test]
fn style_none_suppresses_click_hits() {
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[down(0)], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 100, 100, &mut SpotlightSim::new()).unwrap();
    assert!(s.hits.is_empty() && s.spot.is_some());
}

/// `hits_at` returns RAW virtual-desktop coordinates (`WH_MOUSE_LL`); a window/secondary-monitor
/// capture has a nonzero `ScreenInfo` origin, so a hit must go through `to_frame` (origin
/// subtraction) before `to_panel`, same as every other raw-mouse-point consumer. Without that
/// conversion this Down at the screen's exact origin would land at x = 500/1200 ~= 41.6% across
/// the panel instead of at its top-left corner.
#[test]
fn click_hit_origin_is_converted_before_panel_mapping() {
    let screen = ScreenInfo { w: 1200, h: 800, origin_x: 500, origin_y: 300 };
    let down_at_origin = MouseEvent { t: 0, kind: EventKind::Down, x: 500, y: 300, button: Some(Button::Left) };
    let full = full_scene(1200, 800); // full-frame screen panel: rect = (0,0,1200,800)
    let s = fx_state_at(&fx(ClickFxStyle::Ripple, false), &[down_at_origin], &[], &[], &full, cam(),
        FramePoint { x: 600, y: 400 }, &screen, true, 1200, 800, 0, 0, &mut SpotlightSim::new()).unwrap();
    assert_eq!(s.hits.len(), 1);
    assert!(s.hits[0].x < 5.0 && s.hits[0].y < 5.0,
        "expected the hit at the panel's top-left corner (origin-converted), got ({}, {}) - looks like the raw un-converted 41.6%/37.5% position", s.hits[0].x, s.hits[0].y);
}

#[test]
fn region_and_event_clocks_are_sampled_independently() {
    // Doc effect regions live on the OUTPUT clock, the raw mouse stream on the EVENT clock; a real
    // recording offsets the two by ~800 ms. Sampling the region at 1500 (inside [1000, 2000]) while
    // the click ripple is sampled at 2300 (300 ms after a down at 2000) must leave BOTH active -
    // only possible when the two bases are threaded through separately.
    let e = EffectRegion { id: "e0".into(), kind: EffectKind::Spotlight, start_ms: 1000, end_ms: 2000,
        fade_in_ms: 100, fade_out_ms: 100, mode: None, dim: None, radius: None, feather: None, layer: 0 };
    let s = fx_state_at(&fx(ClickFxStyle::Ripple, false), &[down(2000)], &[], &[e], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, &scr(), true, 100,100, 1500, 2300, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.expect("the region must be sampled at region_t (1500), not ev_t (2300)");
    assert_eq!(s.hits.len(), 1, "the click must be sampled at ev_t (2300), not region_t (1500)");
    // A doc region's shader animation phase follows the clock that drives it (the region clock).
    assert!((spot.t - 1.5).abs() < 1e-6, "spot.t should be region_t seconds: {}", spot.t);
}
