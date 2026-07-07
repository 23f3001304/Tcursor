use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::model::MouseEvent;
use crate::export::camera::ease;
use crate::export::fx::clickfx::hits_at;
use crate::export::coordmap::{project, to_panel};
use crate::export::scene::Scene;
use crate::export::types::{Camera, Easing, FramePoint};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, HotkeySettings, SpotlightMode, VideoFxMode};
use crate::edit::model::{EffectKind, EffectRegion};

/// Click lifetime + spotlight fade, ported verbatim from the old fxdraw overlay.
const LIFE_MS: u32 = 600;
const FADE_MS: u32 = 250;

/// One active click effect in OUTPUT pixels (post-zoom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxHit { pub x: f32, pub y: f32, pub progress: f32 }

/// Active spotlight in OUTPUT pixels. `radius_frac`/`feather_frac` are fractions of output
/// height AS RENDERED - i.e. already pre-scaled by the screen panel's height fraction so the
/// spotlight tracks the screen the cursor lives on rather than the whole frame (see
/// `fx_state_at`; the preview mirrors this by `layout.screen[3]`). `dim` is 0..1, `alpha`
/// scales the whole effect (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub cx: f32, pub cy: f32,
    pub dim: f32, pub radius_frac: f32, pub feather_frac: f32, pub alpha: f32,
    pub mode: SpotlightMode, pub tint: [u8; 3], pub t: f32,
    // Camera PiP exclusion (OUTPUT px): the shader undoes the spotlight dim inside this
    // rounded rect when `dim_camera` is false (the "don't dim the webcam" option).
    pub cam_rect: [f32; 4], pub cam_radius: f32, pub dim_camera: bool,
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

/// One region's own fade-in/out ramp at `et` (0 outside its span), independent of any
/// other region - the building block `SpotlightSim` blends across a handoff.
pub(crate) fn region_alpha(e: &EffectRegion, et: u32) -> f32 {
    if et < e.start_ms || et >= e.end_ms { return 0.0; }
    let inn = (et - e.start_ms) as f32 / e.fade_in_ms.max(1) as f32;
    let outn = (e.end_ms - et) as f32 / e.fade_out_ms.max(1) as f32;
    inn.min(outn).clamp(0.0, 1.0)
}

struct SpotTransition { from_alpha: f32, start_ms: u32, dur_ms: u32 }

/// Stateful spotlight resolver, mirroring `CameraSim`'s pattern: tracks which Spotlight
/// `EffectRegion` (by index into the caller's `effects` slice) is currently the highest-
/// layer active one, and eases alpha across a handoff instead of jump-maxing across
/// overlaps. Style (mode/dim/radius/feather) always comes from the current winner - no
/// blending of two regions' looks simultaneously (override semantics, not compose).
#[derive(Default)]
pub struct SpotlightSim { driver: Option<usize>, transition: Option<SpotTransition>, alpha: f32 }

impl SpotlightSim {
    pub fn new() -> Self { Self::default() }

    fn winner(effects: &[EffectRegion], et: u32) -> Option<usize> {
        effects.iter().enumerate()
            .filter(|(_, e)| matches!(e.kind, EffectKind::Spotlight) && et >= e.start_ms && et < e.end_ms)
            .max_by_key(|(i, e)| (e.layer, *i))
            .map(|(i, _)| i)
    }

    /// Resolves alpha at `et`, unioned with the flat (non-transitioning) settings toggle.
    pub fn resolve(&mut self, effects: &[EffectRegion], et: u32, settings_on: bool) -> f32 {
        let winner_idx = Self::winner(effects, et);
        if winner_idx != self.driver {
            if self.driver.is_some() {
                let dur_ms = match winner_idx {
                    Some(i) => effects[i].fade_in_ms,
                    None => effects[self.driver.unwrap()].fade_out_ms,
                };
                self.transition = Some(SpotTransition { from_alpha: self.alpha, start_ms: et, dur_ms: dur_ms.max(1) });
            }
            self.driver = winner_idx;
        }
        let natural = winner_idx.map(|i| region_alpha(&effects[i], et)).unwrap_or(0.0);
        self.alpha = if let Some(tr) = &self.transition {
            let elapsed = et.saturating_sub(tr.start_ms);
            if elapsed < tr.dur_ms {
                let e = ease(Easing::Smooth, elapsed as f32 / tr.dur_ms as f32);
                tr.from_alpha + (natural - tr.from_alpha) * e
            } else {
                self.transition = None;
                natural
            }
        } else { natural };
        self.alpha.max(if settings_on { 1.0 } else { 0.0 })
    }

    /// The current winner's style, or the settings fallback when no region is active.
    pub fn style(&self, effects: &[EffectRegion], fx: &ClickFxSettings) -> (SpotlightMode, f32, f32, f32) {
        let active_region = self.driver.map(|i| &effects[i]);
        (
            active_region.and_then(|e| e.mode).unwrap_or(fx.spotlight_mode),
            active_region.and_then(|e| e.dim).unwrap_or(fx.spotlight_dim),
            active_region.and_then(|e| e.radius).unwrap_or(fx.spotlight_radius),
            active_region.and_then(|e| e.feather).unwrap_or(fx.spotlight_feather),
        )
    }
}

/// Build the FX state at event-time `et`. `None` when nothing is active (no
/// spotlight and no live clicks) so a renderer can skip the frame entirely.
/// `cur` is the cursor's base/scene point the exporter already computed.
#[allow(clippy::too_many_arguments)]
pub fn fx_state_at(
    fx: &ClickFxSettings, events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion],
    scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, ow: u32, oh: u32, et: u32, spot_sim: &mut SpotlightSim,
) -> Option<FxState> {
    let s_alpha = spot_sim.resolve(effects, et, fx.spotlight);
    let spot = if s_alpha > 0.0 {
        let (cx, cy) = project(cur.x as f32, cur.y as f32, cam, ow, oh);
        let (mode, dim, radius, feather) = spot_sim.style(effects, fx);
        // Size the spotlight relative to the SCREEN PANEL, not the whole output frame: the
        // radius/feather settings are fractions of the screen height, so pre-multiply by the
        // panel's height fraction (`screen.h / oh`) - then the downstream `oh * frac` lands in
        // screen-panel pixels. Without this the spotlight was layout-agnostic (a fixed fraction
        // of the full frame), so a layout that insets/shrinks the screen left the spotlight
        // oversized. The preview mirrors this via `layout.screen[3]`.
        let sfrac = (scene.screen.rect.h / oh.max(1) as f32).max(0.0);
        let cr = scene.camera.rect;
        Some(Spot { cx, cy, dim, radius_frac: radius * sfrac,
            feather_frac: feather * sfrac, alpha: s_alpha,
            mode, tint: fx.spotlight_tint, t: et as f32 / 1000.0,
            cam_rect: [cr.x, cr.y, cr.x + cr.w, cr.y + cr.h],
            cam_radius: scene.camera.radius, dim_camera: fx.spotlight_dim_camera })
    } else { None };

    let mut hits = Vec::new();
    if !matches!(fx.style, ClickFxStyle::None) {
        for h in hits_at(events, et, LIFE_MS) {
            let b = to_panel(FramePoint { x: h.sx, y: h.sy }, sw, sh, scene.screen.rect);
            let (x, y) = project(b.x as f32, b.y as f32, cam, ow, oh);
            hits.push(FxHit { x, y, progress: h.progress });
        }
    }

    let va = crate::export::fx::hold::hold_alpha(actions, et, FADE_MS,
        |k| matches!(k, ActionKind::VideoFxHoldStart),
        |k| matches!(k, ActionKind::VideoFxHoldEnd));
    let video = if va > 0.0 { Some(VideoFx { mode: fx.video_fx_mode, alpha: va, t: et as f32 / 1000.0 }) } else { None };

    if spot.is_none() && hits.is_empty() && video.is_none() { return None; }
    Some(FxState { style: fx.style, color: fx.color, intensity: fx.intensity, hits, spot, video })
}

/// Draws an `FxState` onto a composited BGRA frame.
pub trait FxRenderer: Send {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState);
}

/// Pick the FX renderer: GPU if an adapter is available, else the CPU fallback.
pub fn select_fx(ow: u32, oh: u32) -> Box<dyn FxRenderer> {
    if crate::export::gpu::gpu_available() {
        if let Some(g) = crate::export::fx::fx_gpu::GpuFx::new(ow, oh) { return Box::new(g); }
    }
    Box::new(crate::export::fx::fxdraw::CpuFx)
}

/// Per-frame entry the exporter calls: build state, render it (if any), then captions.
#[allow(clippy::too_many_arguments)]
pub fn render(
    r: &dyn FxRenderer, out: &mut [u8], ow: u32, oh: u32, fx: &ClickFxSettings,
    events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion], scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, et: u32, keys: &HotkeySettings, spot_sim: &mut SpotlightSim,
) {
    if !fx.enabled { return; }
    if let Some(state) = fx_state_at(fx, events, actions, effects, scene, cam, cur, sw, sh, ow, oh, et, spot_sim) {
        r.apply(out, ow, oh, &state);
    }
    crate::export::fx::caption::overlay(out, ow, oh, actions, keys, et, fx.captions);
}

#[cfg(test)]
#[path = "fx_state_tests.rs"]
mod tests;
