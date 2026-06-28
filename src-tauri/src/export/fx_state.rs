use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::model::MouseEvent;
use crate::export::clickfx::hits_at;
use crate::export::coordmap::{project, to_panel};
use crate::export::scene::Scene;
use crate::export::spotlight::hold_alpha;
use crate::export::types::{Camera, FramePoint};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, HotkeySettings, SpotlightMode, VideoFxMode};

/// Click lifetime + spotlight fade, ported verbatim from the old fxdraw overlay.
const LIFE_MS: u32 = 600;
const FADE_MS: u32 = 250;

/// One active click effect in OUTPUT pixels (post-zoom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxHit { pub x: f32, pub y: f32, pub progress: f32 }

/// Active spotlight in OUTPUT pixels. `dim`/`radius_frac`/`feather_frac` are
/// fractions of output height; `alpha` scales the whole effect (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub cx: f32, pub cy: f32,
    pub dim: f32, pub radius_frac: f32, pub feather_frac: f32, pub alpha: f32,
    pub mode: SpotlightMode, pub tint: [u8; 3], pub t: f32,
}

/// Active video FX (full-frame effect triggered by VideoFxHold).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoFx { pub mode: VideoFxMode, pub alpha: f32, pub t: f32 }

/// Renderer-agnostic description of the click-FX + spotlight at one output frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FxState {
    pub style: ClickFxStyle,
    pub color: [u8; 3],
    pub intensity: f32,
    pub hits: Vec<FxHit>,
    pub spot: Option<Spot>,
    pub video: Option<VideoFx>,
}

/// Build the FX state at event-time `et`. `None` when nothing is active (no
/// spotlight and no live clicks) so a renderer can skip the frame entirely.
/// `cur` is the cursor's base/scene point the exporter already computed.
#[allow(clippy::too_many_arguments)]
pub fn fx_state_at(
    fx: &ClickFxSettings, events: &[MouseEvent], actions: &[ActionEvent],
    scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, ow: u32, oh: u32, et: u32,
) -> Option<FxState> {
    let s_alpha = (if fx.spotlight { 1.0_f32 } else { 0.0 }).max(hold_alpha(actions, et, FADE_MS));
    let spot = if s_alpha > 0.0 {
        let (cx, cy) = project(cur.x as f32, cur.y as f32, cam, ow, oh);
        Some(Spot { cx, cy, dim: fx.spotlight_dim, radius_frac: fx.spotlight_radius,
            feather_frac: fx.spotlight_feather, alpha: s_alpha,
            mode: fx.spotlight_mode, tint: fx.spotlight_tint, t: et as f32 / 1000.0 })
    } else { None };

    let mut hits = Vec::new();
    if !matches!(fx.style, ClickFxStyle::None) {
        for h in hits_at(events, et, LIFE_MS) {
            let b = to_panel(FramePoint { x: h.sx, y: h.sy }, sw, sh, scene.screen.rect);
            let (x, y) = project(b.x as f32, b.y as f32, cam, ow, oh);
            hits.push(FxHit { x, y, progress: h.progress });
        }
    }

    let va = crate::export::hold::hold_alpha(actions, et, FADE_MS,
        |k| matches!(k, ActionKind::VideoFxHoldStart),
        |k| matches!(k, ActionKind::VideoFxHoldEnd));
    let video = if va > 0.0 { Some(VideoFx { mode: fx.video_fx_mode, alpha: va, t: et as f32 / 1000.0 }) } else { None };

    if spot.is_none() && hits.is_empty() && video.is_none() { return None; }
    Some(FxState { style: fx.style, color: fx.color, intensity: fx.intensity, hits, spot, video })
}

/// Draws an `FxState` onto a composited BGRA frame.
pub trait FxRenderer {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState);
}

/// Pick the FX renderer: GPU if an adapter is available, else the CPU fallback.
pub fn select_fx(ow: u32, oh: u32) -> Box<dyn FxRenderer> {
    if crate::export::gpu::gpu_available() {
        if let Some(g) = crate::export::fx_gpu::GpuFx::new(ow, oh) { return Box::new(g); }
    }
    Box::new(crate::export::fxdraw::CpuFx)
}

/// Per-frame entry the exporter calls: build state, render it (if any), then captions.
#[allow(clippy::too_many_arguments)]
pub fn render(
    r: &dyn FxRenderer, out: &mut [u8], ow: u32, oh: u32, fx: &ClickFxSettings,
    events: &[MouseEvent], actions: &[ActionEvent], scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, et: u32, keys: &HotkeySettings,
) {
    if !fx.enabled { return; }
    if let Some(state) = fx_state_at(fx, events, actions, scene, cam, cur, sw, sh, ow, oh, et) {
        r.apply(out, ow, oh, &state);
    }
    crate::export::caption::overlay(out, ow, oh, actions, keys, et, fx.captions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind, MouseEvent};
    use crate::actions::model::{ActionEvent, ActionKind};
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
    fn nothing_active_is_none() {
        assert!(fx_state_at(&fx(ClickFxStyle::Ripple, false), &[], &[], &full_scene(100,100), cam(),
            FramePoint{x:50,y:50}, 100,100,100,100, 0).is_none());
    }
    #[test]
    fn spotlight_toggle_makes_a_spot_at_cursor() {
        let s = fx_state_at(&fx(ClickFxStyle::None, true), &[], &[], &full_scene(100,100), cam(),
            FramePoint{x:50,y:50}, 100,100,100,100, 0).unwrap();
        let spot = s.spot.unwrap();
        assert!((spot.alpha - 1.0).abs() < 1e-6);
        assert!((spot.cx - 50.0).abs() < 1.0 && (spot.cy - 50.0).abs() < 1.0);
        assert!(s.hits.is_empty());
    }
    #[test]
    fn a_click_makes_a_hit_in_output_space() {
        let s = fx_state_at(&fx(ClickFxStyle::Ripple, false), &[down(0)], &[], &full_scene(100,100), cam(),
            FramePoint{x:50,y:50}, 100,100,100,100, 300).unwrap();
        assert_eq!(s.hits.len(), 1);
        assert!((s.hits[0].x - 50.0).abs() < 1.0 && s.hits[0].progress > 0.0);
    }
    #[test]
    fn style_none_suppresses_click_hits() {
        let s = fx_state_at(&fx(ClickFxStyle::None, true), &[down(0)], &[], &full_scene(100,100), cam(),
            FramePoint{x:50,y:50}, 100,100,100,100, 100).unwrap();
        assert!(s.hits.is_empty() && s.spot.is_some());
    }
    #[test]
    fn hold_action_ramps_spot_alpha() {
        let acts = vec![ActionEvent { t: 1000, kind: ActionKind::SpotlightHoldStart }];
        let s = fx_state_at(&fx(ClickFxStyle::None, false), &[], &acts, &full_scene(100,100), cam(),
            FramePoint{x:50,y:50}, 100,100,100,100, 1125).unwrap(); // 125ms into 250ms ramp
        assert!((s.spot.unwrap().alpha - 0.5).abs() < 0.05);
    }
}
