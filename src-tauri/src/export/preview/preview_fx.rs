use crate::export::fx::fx_state::{select_fx, FxHit, FxRenderer, FxState, Spot, VideoFx};
use crate::export::preview::{base64_encode, png_encode, PreviewSession};
use crate::settings::model::{ClickFxStyle, SpotlightMode, VideoFxMode};
use std::sync::{Mutex, OnceLock};

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

fn reconstruct(on_black: &[u8], on_white: &[u8]) -> Vec<u8> {
    let mut overlay = vec![0u8; on_black.len()];
    for i in (0..on_black.len()).step_by(4) {
        let mut a = 0.0f32;
        for c in 0..3 {
            let d = (on_white[i + c] as f32 - on_black[i + c] as f32).max(0.0);
            a = a.max(1.0 - d / 255.0);
        }
        if a <= 0.002 {
            continue;
        }
        for c in 0..3 {
            overlay[i + c] = (on_black[i + c] as f32 / a).clamp(0.0, 255.0) as u8;
        }
        overlay[i + 3] = (a * 255.0).round() as u8;
    }
    overlay
}

fn gate_cam_hole(
    has_webcam: bool,
    cam_rect: Option<[f32; 4]>,
    cam_radius: Option<f32>,
    dim_camera: Option<bool>,
) -> (Option<[f32; 4]>, Option<f32>, Option<bool>) {
    if cam_rect.is_some() && !has_webcam {
        (None, None, Some(true))
    } else {
        (cam_rect, cam_radius, dim_camera)
    }
}

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

#[allow(clippy::too_many_arguments)]
fn render_fx_overlay(
    fx: &dyn FxRenderer,
    ow: u32,
    oh: u32,
    has_webcam: bool,
    style: String,
    color: [u8; 3],
    intensity: f32,
    hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>,
    spot_cy: Option<f32>,
    spot_dim: Option<f32>,
    spot_radius: Option<f32>,
    spot_feather: Option<f32>,
    spot_alpha: Option<f32>,
    spot_mode: Option<String>,
    spot_tint: Option<[u8; 3]>,
    spot_t: Option<f32>,
    video_mode: Option<String>,
    video_alpha: Option<f32>,
    video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>,
    cam_radius: Option<f32>,
    dim_camera: Option<bool>,
) -> Result<String, String> {
    let (ow, oh) = (ow.max(1), oh.max(1));
    let (cam_rect, cam_radius, dim_camera) =
        gate_cam_hole(has_webcam, cam_rect, cam_radius, dim_camera);
    let spot = match (spot_cx, spot_cy, spot_alpha) {
        (Some(cx), Some(cy), Some(alpha)) if alpha > 0.0 => Some(Spot {
            cx,
            cy,
            dim: spot_dim.unwrap_or(0.6),
            radius_frac: spot_radius.unwrap_or(0.13),
            feather_frac: spot_feather.unwrap_or(0.10),
            alpha,
            mode: spot_mode
                .as_deref()
                .map(spot_mode_of)
                .unwrap_or(SpotlightMode::Classic),
            tint: spot_tint.unwrap_or([130, 90, 255]),
            t: spot_t.unwrap_or(0.0),
            cam_rect: cam_rect.unwrap_or([0.0; 4]),
            cam_radius: cam_radius.unwrap_or(0.0),
            dim_camera: dim_camera.unwrap_or(true),
        }),
        _ => None,
    };
    let video = match (video_alpha, video_t) {
        (Some(alpha), Some(t)) if alpha > 0.0 => Some(VideoFx {
            mode: video_mode
                .as_deref()
                .map(video_mode_of)
                .unwrap_or(VideoFxMode::NebulaWash),
            alpha,
            t,
        }),
        _ => None,
    };
    let hits: Vec<FxHit> = hits
        .iter()
        .map(|h| FxHit {
            x: h[0],
            y: h[1],
            progress: h[2],
        })
        .collect();
    let state = FxState {
        style: click_style_of(&style),
        color,
        intensity,
        hits,
        spot,
        video,
        ..Default::default()
    };

    let n = (ow * oh * 4) as usize;
    let mut on_black = vec![0u8; n];
    for p in on_black.chunks_exact_mut(4) {
        p[3] = 255;
    }
    let mut on_white = vec![255u8; n];
    fx.apply(&mut on_black, ow, oh, &state);
    fx.apply(&mut on_white, ow, oh, &state);

    let overlay = reconstruct(&on_black, &on_white);
    let png = png_encode(&overlay, ow, oh).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn preview_fx_overlay(
    ow: u32,
    oh: u32,
    style: String,
    color: [u8; 3],
    intensity: f32,
    hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>,
    spot_cy: Option<f32>,
    spot_dim: Option<f32>,
    spot_radius: Option<f32>,
    spot_feather: Option<f32>,
    spot_alpha: Option<f32>,
    spot_mode: Option<String>,
    spot_tint: Option<[u8; 3]>,
    spot_t: Option<f32>,
    video_mode: Option<String>,
    video_alpha: Option<f32>,
    video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>,
    cam_radius: Option<f32>,
    dim_camera: Option<bool>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let (ow, oh) = (ow.max(1), oh.max(1));
        let has_webcam = app.state::<PreviewSession>().has_webcam();
        with_fx(ow, oh, |fx| {
            render_fx_overlay(
                fx,
                ow,
                oh,
                has_webcam,
                style,
                color,
                intensity,
                hits,
                spot_cx,
                spot_cy,
                spot_dim,
                spot_radius,
                spot_feather,
                spot_alpha,
                spot_mode,
                spot_tint,
                spot_t,
                video_mode,
                video_alpha,
                video_t,
                cam_rect,
                cam_radius,
                dim_camera,
            )
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "preview_fx_tests.rs"]
mod tests;
