// The editor preview's FX overlay as a standalone transparent PNG (spotlight + click effects),
// blitted over the JS-composited base frame. The frontend has already resolved every value
// (spotlight centre/radius/feather/alpha and click hits, all in FX-canvas pixels), so this builds
// an `FxState` straight from the params - no events/actions/scene - and runs the exact `CpuFx`
// primitives the export uses. Those primitives composite ONTO an opaque frame (multiply-dim +
// additive tint, in place), so there is no source alpha to read back: instead we render the effect
// twice - over solid black and over solid white - and invert the "over" composite per pixel,
// `a = 1 - (white - black)/255`, straight colour = black / a. Without this command the
// `preview_fx_overlay` invoke rejected (it was never registered), `fxOverlay.ts` swallowed the
// error and returned null, so the spotlight never showed in the preview.
use crate::export::fx::fx_state::{FxHit, FxRenderer, FxState, Spot, VideoFx};
use crate::export::fx::fxdraw::CpuFx;
use crate::export::preview::{base64_encode, png_encode};
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

/// Tauri command: render the frontend-resolved FX overlay and return a transparent PNG data URL.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn preview_fx_overlay(
    ow: u32, oh: u32,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
) -> Result<String, String> {
    let (ow, oh) = (ow.max(1), oh.max(1));
    let spot = match (spot_cx, spot_cy, spot_alpha) {
        (Some(cx), Some(cy), Some(alpha)) if alpha > 0.0 => Some(Spot {
            cx, cy, dim: spot_dim.unwrap_or(0.6),
            radius_frac: spot_radius.unwrap_or(0.13), feather_frac: spot_feather.unwrap_or(0.10),
            alpha, mode: spot_mode.as_deref().map(spot_mode_of).unwrap_or(SpotlightMode::Classic),
            tint: spot_tint.unwrap_or([130, 90, 255]), t: spot_t.unwrap_or(0.0),
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
    CpuFx.apply(&mut on_black, ow, oh, &state);
    CpuFx.apply(&mut on_white, ow, oh, &state);

    let overlay = reconstruct(&on_black, &on_white);
    let png = png_encode(&overlay, ow, oh).map_err(|e| e.to_string())?;
    Ok(format!("data:image/png;base64,{}", base64_encode(&png)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_area_becomes_black_with_matching_alpha() {
        // A pixel dimmed to 40% (m=0.4): black stays 0, white -> 102. a = 1-0.4 = 0.6.
        let o = reconstruct(&[0, 0, 0, 255], &[102, 102, 102, 255]);
        assert_eq!((o[0], o[1], o[2]), (0, 0, 0), "dim overlay colour is black");
        assert!((o[3] as i32 - 153).abs() <= 1, "alpha ~0.6 -> 153, got {}", o[3]);
    }

    #[test]
    fn untouched_pixel_is_fully_transparent() {
        let o = reconstruct(&[0, 0, 0, 255], &[255, 255, 255, 255]);
        assert_eq!(o[3], 0, "a pixel the effect never touched must be transparent");
    }

    #[test]
    fn additive_click_recovers_bright_colour() {
        // additive c=128: black -> 128, white saturates -> 255. a = 128/255 ~ 0.502.
        let o = reconstruct(&[128, 128, 128, 255], &[255, 255, 255, 255]);
        assert!(o[3] > 120 && o[3] < 135, "alpha ~0.5, got {}", o[3]);
        assert!(o[0] > 240 && o[1] > 240 && o[2] > 240, "colour recovered near full");
    }

    #[test]
    fn spotlight_command_returns_a_png_data_url() {
        let url = preview_fx_overlay(
            80, 80, "none".into(), [255, 255, 255], 1.0, vec![],
            Some(40.0), Some(40.0), Some(0.6), Some(0.13), Some(0.10), Some(1.0),
            Some("classic".into()), Some([130, 90, 255]), Some(0.0),
            None, None, None,
        ).unwrap();
        assert!(url.starts_with("data:image/png;base64,"), "returns a PNG data URL");
        assert!(url.len() > 200, "non-trivial overlay encoded");
    }
}
