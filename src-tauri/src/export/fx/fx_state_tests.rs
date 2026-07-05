// Tests for export::fx::fx_state, split into their own file so fx_state.rs stays under the size limit.
use super::*;
use crate::events::model::{Button, EventKind, MouseEvent};
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, FramePoint, RectF};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, SpotlightMode};

fn full_scene(w: u32, h: u32) -> Scene {
    Scene {
        screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: w as f32, h: h as f32 }, radius: 0.0, alpha: 1.0 },
        camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0 },
    }
}
fn fx(style: ClickFxStyle, spot: bool) -> ClickFxSettings {
    ClickFxSettings { enabled: true, style, color: [255, 0, 0], intensity: 1.0, captions: false,
        spotlight: spot, spotlight_dim: 0.6, spotlight_radius: 0.13, spotlight_feather: 0.10,
        spotlight_mode: SpotlightMode::Classic, spotlight_tint: [130, 90, 255],
        video_fx_mode: crate::settings::model::VideoFxMode::NebulaWash }
}
fn cam() -> Camera { Camera { cx: 50.0, cy: 50.0, scale: 1.0 } }
fn down(t: u32) -> MouseEvent { MouseEvent { t, kind: EventKind::Down, x: 50, y: 50, button: Some(Button::Left) } }

#[test]
fn spotlight_uses_per_region_fades() {
    let e = EffectRegion { id: "e0".into(), kind: EffectKind::Spotlight, start_ms: 0, end_ms: 1000, fade_in_ms: 100, fade_out_ms: 500, mode: None, dim: None, radius: None, feather: None, layer: 0 };
    // 50 ms into a 100 ms fade-in -> ~0.5
    let a = super::region_alpha(&e, 50);
    assert!((a - 0.5).abs() < 0.05, "fade-in half way: {a}");
    // 250 ms before the end of a 500 ms fade-out -> ~0.5
    let b = super::region_alpha(&e, 750);
    assert!((b - 0.5).abs() < 0.05, "fade-out half way: {b}");
}

#[test]
fn nothing_active_is_none() {
    assert!(fx_state_at(&fx(ClickFxStyle::Ripple, false), &[], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, 100,100,100,100, 0, &mut SpotlightSim::new()).is_none());
}
#[test]
fn spotlight_toggle_makes_a_spot_at_cursor() {
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, 100,100,100,100, 0, &mut SpotlightSim::new()).unwrap();
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
        screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: 100.0, h: 50.0 }, radius: 0.0, alpha: 1.0 },
        camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0 },
    };
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[], &[], &[], &half, cam(),
        FramePoint{x:50,y:50}, 100,100,100,100, 0, &mut SpotlightSim::new()).unwrap();
    let spot = s.spot.unwrap();
    // screen.h / oh = 50/100 = 0.5, so radius/feather are half their settings-space value.
    assert!((spot.radius_frac - 0.13 * 0.5).abs() < 1e-6, "radius_frac should scale by screen height fraction: {}", spot.radius_frac);
    assert!((spot.feather_frac - 0.10 * 0.5).abs() < 1e-6, "feather_frac should scale too: {}", spot.feather_frac);
}
#[test]
fn a_click_makes_a_hit_in_output_space() {
    let s = fx_state_at(&fx(ClickFxStyle::Ripple, false), &[down(0)], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, 100,100,100,100, 300, &mut SpotlightSim::new()).unwrap();
    assert_eq!(s.hits.len(), 1);
    assert!((s.hits[0].x - 50.0).abs() < 1.0 && s.hits[0].progress > 0.0);
}
#[test]
fn style_none_suppresses_click_hits() {
    let s = fx_state_at(&fx(ClickFxStyle::None, true), &[down(0)], &[], &[], &full_scene(100,100), cam(),
        FramePoint{x:50,y:50}, 100,100,100,100, 100, &mut SpotlightSim::new()).unwrap();
    assert!(s.hits.is_empty() && s.spot.is_some());
}

#[test]
fn highest_layer_region_wins_style_not_first_match() {
    let a = EffectRegion { id: "e0".into(), kind: EffectKind::Spotlight, start_ms: 0, end_ms: 2000,
        fade_in_ms: 100, fade_out_ms: 100, mode: Some(SpotlightMode::Classic), dim: None, radius: None, feather: None, layer: 0 };
    let b = EffectRegion { id: "e1".into(), kind: EffectKind::Spotlight, start_ms: 0, end_ms: 2000,
        fade_in_ms: 100, fade_out_ms: 100, mode: Some(SpotlightMode::Nebula), dim: None, radius: None, feather: None, layer: 1 };
    let mut sim = SpotlightSim::new();
    sim.resolve(&[a.clone(), b.clone()], 0, false);
    let (mode, _, _, _) = sim.style(&[a, b], &fx(ClickFxStyle::None, false));
    assert_eq!(mode, SpotlightMode::Nebula, "the higher-layer region (b) should win, not the first match (a)");
}

#[test]
fn spotlight_handoff_eases_alpha_instead_of_jump_maxing() {
    // a: layer 0, active the whole time, long fade so it's fully faded in by t=500.
    // b: layer 1 (wins), active only 500..600, with a slow 300ms fade-in.
    // At t=520 (20ms into b's window), alpha should be close to a's already-faded-in
    // level (near 1.0), not b's own barely-started fade (which alone would be ~0.07).
    let a = EffectRegion { id: "e0".into(), kind: EffectKind::Spotlight, start_ms: 0, end_ms: 2000,
        fade_in_ms: 100, fade_out_ms: 100, mode: None, dim: None, radius: None, feather: None, layer: 0 };
    let b = EffectRegion { id: "e1".into(), kind: EffectKind::Spotlight, start_ms: 500, end_ms: 600,
        fade_in_ms: 300, fade_out_ms: 300, mode: None, dim: None, radius: None, feather: None, layer: 1 };
    let mut sim = SpotlightSim::new();
    let mut alpha = 0.0;
    for et in (0..500).step_by(16) { alpha = sim.resolve(&[a.clone(), b.clone()], et, false); }
    assert!(alpha > 0.9, "a should be fully faded in by t=500: {alpha}");
    let after = sim.resolve(&[a, b], 520, false);
    assert!(after > 0.8, "alpha should not jump-drop toward b's own barely-started fade: {after}");
}
