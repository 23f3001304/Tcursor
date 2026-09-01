// The editor preview's FX overlay as a standalone transparent PNG (spotlight + click effects),
// blitted over the JS-composited base frame. The frontend has already resolved every value
// (spotlight centre/radius/feather/alpha and click hits, all in FX-canvas pixels), so this builds
// an `FxState` straight from the params - no events/actions/scene - and renders it through
// `select_fx`, the EXPORT's own renderer selector (see `with_fx`), so preview and export run the
// same shader/primitives per effect. Those primitives composite ONTO an opaque frame (multiply-dim +
// additive tint, in place), so there is no source alpha to read back: instead we render the effect
// twice - over solid black and over solid white - and invert the "over" composite per pixel,
// `a = 1 - (white - black)/255`, straight colour = black / a. Without this command the
// `preview_fx_overlay` invoke rejected (it was never registered), `fxOverlay.ts` swallowed the
// error and returned null, so the spotlight never showed in the preview.
//
// One exception to "pure function of its params": the spotlight's camera-exclusion hole
// (`cam_rect`/`cam_radius`/`dim_camera`) is gated on `PreviewSession::has_webcam` - the frontend
// has no reliable way to know whether `webcam.webm` actually exists on disk, so without this the
// preview could show an un-dimmed hole for a layout with a camera panel but no recorded webcam,
// which the export (gated the same way in `fx_state.rs`) never shows. `gate_cam_hole` is the pure
// part of that; `render_fx_overlay` takes the resolved `has_webcam` bool directly, so it stays
// testable without a warm `PreviewSession` - see `preview_fx_tests.rs`.
use std::sync::{Mutex, OnceLock};
use crate::export::fx::fx_state::{select_fx, FxHit, FxRenderer, FxState, Spot, VideoFx};
use crate::export::preview::{base64_encode, png_encode, PreviewSession};
use crate::settings::model::{ClickFxStyle, SpotlightMode, VideoFxMode};

fn click_style_of(s: &str) -> ClickFxStyle {
    match s {
        "ripple" => ClickFxStyle::Ripple,
        "pulse" => ClickFxStyle::Pulse,
        "glow" => ClickFxStyle::Glow,
        "shockwave" => ClickFxStyle::Shockwave,
        "particles" => ClickFxStyle::Particles,
        "neon" => ClickFxStyle::Neon,
        _ => ClickFxStyle::None,
    }
}

fn spot_mode_of(s: &str) -> SpotlightMode {
    match s {
        "blur" => SpotlightMode::Blur,
        "halo" => SpotlightMode::Halo,
        "breathing" => SpotlightMode::Breathing,
        "nebula" => SpotlightMode::Nebula,
        "vignette" => SpotlightMode::Vignette,
        _ => SpotlightMode::Classic,
    }
}

fn video_mode_of(s: &str) -> VideoFxMode {
    match s {
        "cinematicdim" | "cinematic_dim" => VideoFxMode::CinematicDim,
        "screenfocus" | "screen_focus" => VideoFxMode::ScreenFocus,
        "colorpop" | "color_pop" => VideoFxMode::ColorPop,
        _ => VideoFxMode::NebulaWash,
    }
}

/// Recover a straight-alpha BGRA overlay from the same effect rendered over black and white.
fn reconstruct(on_black: &[u8], on_white: &[u8]) -> Vec<u8> {
    let mut overlay = vec![0u8; on_black.len()];
    for i in (0..on_black.len()).step_by(4) {
        // The over-composite `out = bg*(1-a) + col*a` gives `white - black = 255*(1-a)` per
        // channel; take the strongest-touched channel as the pixel's alpha.
        let mut a = 0.0f32;
        for c in 0..3 {
            let d = (on_white[i + c] as f32 - on_black[i + c] as f32).max(0.0);
            a = a.max(1.0 - d / 255.0);
        }
        if a <= 0.002 { continue; } // pixel the effect never touched -> fully transparent
        for c in 0..3 {
            overlay[i + c] = (on_black[i + c] as f32 / a).clamp(0.0, 255.0) as u8; // col = black / a
        }
        overlay[i + 3] = (a * 255.0).round() as u8;
    }
    overlay
}

/// Whether the spotlight's camera-exclusion hole should apply this frame - `has_webcam` mirrors
/// the export's `has_hole = has_webcam && scene.camera.alpha > 0.05` gate (`fx_state.rs`); the
/// preview has no `scene.camera.alpha` to check (the frontend never resolves one), so a requested
/// hole (`cam_rect.is_some()`) is the preview's equivalent signal of "a camera panel wants a hole
/// here". When the gate fails, the rect/radius are dropped and `dim_camera` forced to `true`
/// (belt-and-suspenders, same as `fx_state_at`'s zeroed representation) so a stray `false` from
/// the frontend can never leave an un-dimmed rectangle over a project with no recorded webcam.
fn gate_cam_hole(
    has_webcam: bool, cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
) -> (Option<[f32; 4]>, Option<f32>, Option<bool>) {
    if cam_rect.is_some() && !has_webcam { (None, None, Some(true)) } else { (cam_rect, cam_radius, dim_camera) }
}

/// Run `f` with the FX renderer the preview overlay draws with, cached for one output size.
/// The renderer is MOVED out of the cache for the duration of the call, never borrowed from
/// under the guard - see the notes in `preview_fx.md` on why the lock must not span `f`.
pub(crate) fn with_fx<T>(ow: u32, oh: u32, f: impl FnOnce(&dyn FxRenderer) -> T) -> T {
    static FX: OnceLock<Mutex<Option<(u32, u32, Box<dyn FxRenderer>)>>> = OnceLock::new();
    let cell = FX.get_or_init(|| Mutex::new(None));
    let lock = || cell.lock().unwrap_or_else(|e| e.into_inner());
    let cached = lock().take().filter(|(w, h, _)| *w == ow && *h == oh);
    let (w, h, fx) = cached.unwrap_or_else(|| (ow, oh, select_fx(ow, oh)));
    let out = f(fx.as_ref());
    *lock() = Some((w, h, fx));
    out
}

/// Render the frontend-resolved FX overlay into a transparent PNG data URL. Pure aside from
/// `has_webcam` (resolved by the caller from `PreviewSession`), so it's directly unit-testable.
#[allow(clippy::too_many_arguments)]
fn render_fx_overlay(
    fx: &dyn FxRenderer, ow: u32, oh: u32, has_webcam: bool,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
) -> Result<String, String> {
    let (ow, oh) = (ow.max(1), oh.max(1));
    let (cam_rect, cam_radius, dim_camera) = gate_cam_hole(has_webcam, cam_rect, cam_radius, dim_camera);
    let spot = match (spot_cx, spot_cy, spot_alpha) {
        (Some(cx), Some(cy), Some(alpha)) if alpha > 0.0 => Some(Spot {
            cx, cy, dim: spot_dim.unwrap_or(0.6),
            radius_frac: spot_radius.unwrap_or(0.13), feather_frac: spot_feather.unwrap_or(0.10),
            alpha, mode: spot_mode.as_deref().map(spot_mode_of).unwrap_or(SpotlightMode::Classic),
            tint: spot_tint.unwrap_or([130, 90, 255]), t: spot_t.unwrap_or(0.0),
            cam_rect: cam_rect.unwrap_or([0.0; 4]), cam_radius: cam_radius.unwrap_or(0.0),
            dim_camera: dim_camera.unwrap_or(true),
        }),
        _ => None,
    };
    let video = match (video_alpha, video_t) {
        (Some(alpha), Some(t)) if alpha > 0.0 =>
            Some(VideoFx { mode: video_mode.as_deref().map(video_mode_of).unwrap_or(VideoFxMode::NebulaWash), alpha, t }),
        _ => None,
    };
    let hits: Vec<FxHit> = hits.iter().map(|h| FxHit { x: h[0], y: h[1], progress: h[2] }).collect();
    let state = FxState { style: click_style_of(&style), color, intensity, hits, spot, video };

    // Render onto opaque black and opaque white, then invert the over-composite for alpha.
    let n = (ow * oh * 4) as usize;
    let mut on_black = vec![0u8; n];
    for p in on_black.chunks_exact_mut(4) { p[3] = 255; }
    let mut on_white = vec![255u8; n];
    fx.apply(&mut on_black, ow, oh, &state);
    fx.apply(&mut on_white, ow, oh, &state);

    let overlay = reconstruct(&on_black, &on_white);
    let png = png_encode(&overlay, ow, oh).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

/// Tauri command: render the frontend-resolved FX overlay and return a transparent PNG data URL.
/// A thin wrapper over `render_fx_overlay` - its only job is resolving `has_webcam` from the warm
/// `PreviewSession` (whichever project is currently open; see `PreviewSession::has_webcam`).
///
/// `async` + `spawn_blocking` (see `preview_frame` for the pattern and the `AppHandle`-instead-of-
/// `State` reason). This is the app's hottest command - the composite loop fires it ~25x/sec for
/// the whole of playback and every scrub - and each call does two full GPU render+readback fences
/// (`device.poll(Maintain::Wait)`, which drains the process-shared device queue), a per-pixel
/// alpha reconstruction, a PNG deflate and a base64 encode. As a sync command all of that ran on
/// the main thread, which is what made the whole window unresponsive during playback and
/// dramatically worse while an export was pushing work onto the same wgpu device.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn preview_fx_overlay(
    ow: u32, oh: u32,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let (ow, oh) = (ow.max(1), oh.max(1));
        let has_webcam = app.state::<PreviewSession>().has_webcam();
        with_fx(ow, oh, |fx| render_fx_overlay(fx, ow, oh, has_webcam, style, color, intensity, hits,
            spot_cx, spot_cy, spot_dim, spot_radius, spot_feather, spot_alpha, spot_mode, spot_tint, spot_t,
            video_mode, video_alpha, video_t, cam_rect, cam_radius, dim_camera))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "preview_fx_tests.rs"]
mod tests;
