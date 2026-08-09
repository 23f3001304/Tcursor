use crate::actions::model::{ActionEvent, ActionKind};
use crate::events::model::{MouseEvent, ScreenInfo};
use crate::export::fx::clickfx::hits_at;
use crate::export::coordmap::{project, to_frame, to_panel};
use crate::export::scene::Scene;
use crate::export::types::{Camera, FramePoint};
use crate::settings::model::{ClickFxSettings, ClickFxStyle, HotkeySettings, SpotlightMode, VideoFxMode};
use crate::edit::model::EffectRegion;
// Split out so this file stays under the size limit (`SpotlightSim` re-exported so callers keep
// using `fx_state::`; `region_alpha` is internal to `SpotlightSim` now - its own tests reach it
// via `crate::export::fx::spotlight_sim::region_alpha` directly).
pub use crate::export::fx::spotlight_sim::SpotlightSim;

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

/// Build the FX state for one frame. TWO clocks: `region_t` is output time (0 = first video
/// frame), the clock every `EditDoc` region list lives on; `ev_t` is event time, the clock the raw
/// mouse/action streams are recorded on. Doc `effects` are sampled at `region_t`, clicks and
/// hold-driven video FX at `ev_t`. `None` when nothing is active (no spotlight and no live clicks)
/// so a renderer can skip the frame entirely. `cur` is the cursor point the exporter computed.
/// `screen` is the recording's `ScreenInfo` (virtual-desktop origin) - `hits_at` returns raw
/// `WH_MOUSE_LL` desktop coordinates, so each hit is converted through `to_frame` before the
/// `to_panel` screen-content mapping, same as every other consumer of a raw mouse point.
/// `has_webcam` is whether `webcam.mp4` exists on disk for this recording - the "keep camera lit"
/// hole must never apply without it (nor when the camera panel itself isn't visible this frame),
/// or an un-dimmed empty rectangle appears in ScreenOnly layouts, after Hide, or when there was
/// never a webcam at all.
#[allow(clippy::too_many_arguments)]
pub fn fx_state_at(
    fx: &ClickFxSettings, events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion],
    scene: &Scene, cam: Camera, cur: FramePoint, screen: &ScreenInfo, has_webcam: bool,
    sw: u32, sh: u32, ow: u32, oh: u32, region_t: u32, ev_t: u32, spot_sim: &mut SpotlightSim,
) -> Option<FxState> {
    let s_alpha = spot_sim.resolve(effects, region_t, fx.spotlight);
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
        // "No hole" representation: `dim_camera: true` forces `draw_spot`/the GPU shader to skip
        // the un-dim entirely (matches today's behavior when the toggle is on), regardless of the
        // user's actual `spotlight_dim_camera` setting - so a missing/invisible webcam can never
        // leave an un-dimmed empty rectangle. `cam_rect`/`cam_radius` are zeroed too, belt-and-
        // suspenders against a future reader that checks the rect instead of the flag.
        let has_hole = has_webcam && scene.camera.alpha > 0.05;
        Some(Spot { cx, cy, dim, radius_frac: radius * sfrac,
            feather_frac: feather * sfrac, alpha: s_alpha,
            // Animation phase (breathing/nebula) follows the clock that DRIVES the effect, so the
            // TS preview - which passes its output-time `now` as `spotT` - renders the same phase.
            mode, tint: fx.spotlight_tint, t: region_t as f32 / 1000.0,
            cam_rect: if has_hole { [cr.x, cr.y, cr.x + cr.w, cr.y + cr.h] } else { [0.0; 4] },
            cam_radius: if has_hole { scene.camera.radius } else { 0.0 },
            dim_camera: if has_hole { fx.spotlight_dim_camera } else { true } })
    } else { None };

    let mut hits = Vec::new();
    if !matches!(fx.style, ClickFxStyle::None) {
        for h in hits_at(events, ev_t, LIFE_MS) {
            let p = to_frame(screen, h.sx, h.sy);
            let b = to_panel(p, sw, sh, scene.screen.rect);
            let (x, y) = project(b.x as f32, b.y as f32, cam, ow, oh);
            hits.push(FxHit { x, y, progress: h.progress });
        }
    }

    // Video FX is a raw hotkey hold, not a doc region: both its alpha and its phase are event-time.
    let va = crate::export::fx::hold::hold_alpha(actions, ev_t, FADE_MS,
        |k| matches!(k, ActionKind::VideoFxHoldStart),
        |k| matches!(k, ActionKind::VideoFxHoldEnd));
    let video = if va > 0.0 { Some(VideoFx { mode: fx.video_fx_mode, alpha: va, t: ev_t as f32 / 1000.0 }) } else { None };

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

/// Per-frame entry the exporter calls: build state, render it (if any), then captions. `region_t`
/// (output clock) drives the doc's effect regions; `ev_t` (event clock) drives the raw streams -
/// click ripples, hold-driven video FX and the hotkey captions. `screen`/`has_webcam` are threaded
/// straight to `fx_state_at` (see its doc) for the click-hit origin conversion and the spotlight
/// camera-exclusion hole gate, respectively.
#[allow(clippy::too_many_arguments)]
pub fn render(
    r: &dyn FxRenderer, out: &mut [u8], ow: u32, oh: u32, fx: &ClickFxSettings,
    events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion], scene: &Scene, cam: Camera, cur: FramePoint, screen: &ScreenInfo, has_webcam: bool,
    sw: u32, sh: u32, region_t: u32, ev_t: u32, keys: &HotkeySettings, spot_sim: &mut SpotlightSim,
) {
    if !fx.enabled { return; }
    if let Some(state) = fx_state_at(fx, events, actions, effects, scene, cam, cur, screen, has_webcam, sw, sh, ow, oh, region_t, ev_t, spot_sim) {
        r.apply(out, ow, oh, &state);
    }
    crate::export::fx::caption::overlay(out, ow, oh, actions, keys, ev_t, fx.captions);
}

#[cfg(test)]
#[path = "fx_state_tests.rs"]
mod tests;
