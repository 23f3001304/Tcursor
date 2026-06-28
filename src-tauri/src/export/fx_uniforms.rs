use crate::export::fx_state::FxState;
use crate::settings::model::{ClickFxStyle, SpotlightMode, VideoFxMode};

/// Max simultaneous click hits passed to the shader (older clicks beyond this are dropped).
pub const MAX_HITS: usize = 16;

/// FX shader uniform. All vec4-aligned for std140. Fields are logical RGBA space
/// (the Bgra8Unorm format handles byte order on sample + store).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FxU {
    pub a: [f32; 4],                // ow, oh, style(0..6), hit_count
    pub b: [f32; 4],                // spot_cx, spot_cy, spot_dim*alpha, spot_active(0/1)
    pub c: [f32; 4],                // spot_r_in(px), spot_r_out(px), intensity, _pad
    pub d: [f32; 4],                // spot_mode_id, time_s, _pad, _pad
    pub tint: [f32; 4],             // r, g, b (0..1), _pad
    pub color: [f32; 4],            // r, g, b (0..1), _pad
    pub hits: [[f32; 4]; MAX_HITS], // x, y, progress, _pad
    pub e: [f32; 4],                // video_mode_id, alpha, t, _pad
}

/// The shader's style id for a click style. SINGLE SOURCE OF TRUTH - the
/// `FX_*` consts in fx.wgsl must mirror these exactly.
pub fn style_id(s: ClickFxStyle) -> f32 {
    match s {
        ClickFxStyle::None => 0.0, ClickFxStyle::Ripple => 1.0, ClickFxStyle::Pulse => 2.0,
        ClickFxStyle::Glow => 3.0, ClickFxStyle::Shockwave => 4.0,
        ClickFxStyle::Particles => 5.0, ClickFxStyle::Neon => 6.0,
    }
}

/// Numeric id for a video FX mode passed to the shader in FxU.e[0].
pub fn video_mode_id(m: VideoFxMode) -> f32 {
    match m {
        VideoFxMode::NebulaWash => 0.0, VideoFxMode::CinematicDim => 1.0,
        VideoFxMode::ScreenFocus => 2.0, VideoFxMode::ColorPop => 3.0,
    }
}

/// Numeric id for a spotlight mode passed to the shader in FxU.d[0].
pub fn spot_mode_id(m: SpotlightMode) -> f32 {
    match m {
        SpotlightMode::Classic => 0.0, SpotlightMode::Blur => 1.0, SpotlightMode::Halo => 2.0,
        SpotlightMode::Breathing => 3.0, SpotlightMode::Nebula => 4.0, SpotlightMode::Vignette => 5.0,
    }
}

/// Pack an `FxState` into the shader uniform (output pixels).
pub fn build_fx_u(state: &FxState, ow: u32, oh: u32) -> FxU {
    let style = style_id(state.style);
    let mut hits = [[0.0f32; 4]; MAX_HITS];
    for (i, h) in state.hits.iter().take(MAX_HITS).enumerate() {
        hits[i] = [h.x, h.y, h.progress, 0.0];
    }
    let n = state.hits.len().min(MAX_HITS) as f32;
    let (b, c) = match state.spot {
        Some(s) => {
            let r_in = oh as f32 * s.radius_frac.max(0.0);
            let r_out = r_in + oh as f32 * s.feather_frac.max(0.001);
            ([s.cx, s.cy, s.dim.clamp(0.0, 1.0) * s.alpha.clamp(0.0, 1.0), 1.0],
             [r_in, r_out, state.intensity, 0.0])
        }
        None => ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, state.intensity, 0.0]),
    };
    let (d, tint) = match state.spot {
        Some(s) => ([spot_mode_id(s.mode), s.t, 0.0, 0.0],
                    [s.tint[0] as f32 / 255.0, s.tint[1] as f32 / 255.0, s.tint[2] as f32 / 255.0, 0.0]),
        None => ([0.0; 4], [0.0; 4]),
    };
    let e = match state.video {
        Some(v) => [video_mode_id(v.mode), v.alpha, v.t, 0.0],
        None => [0.0; 4],
    };
    FxU {
        a: [ow as f32, oh as f32, style, n], b, c, d, tint,
        color: [state.color[0] as f32 / 255.0, state.color[1] as f32 / 255.0, state.color[2] as f32 / 255.0, 0.0],
        hits, e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fx_state::{FxHit, FxState, Spot};
    use crate::settings::model::{ClickFxStyle, SpotlightMode};

    #[test]
    fn style_id_covers_all_variants() {
        use crate::settings::model::ClickFxStyle::*;
        assert_eq!(style_id(None), 0.0);
        assert_eq!(style_id(Ripple), 1.0);
        assert_eq!(style_id(Pulse), 2.0);
        assert_eq!(style_id(Glow), 3.0);
        assert_eq!(style_id(Shockwave), 4.0);
        assert_eq!(style_id(Particles), 5.0);
        assert_eq!(style_id(Neon), 6.0);
    }
    #[test]
    fn maps_style_hits_spot_and_color() {
        let st = FxState { style: ClickFxStyle::Ripple, color: [255, 0, 0], intensity: 0.8,
            hits: vec![FxHit { x: 10.0, y: 20.0, progress: 0.4 }],
            spot: Some(Spot { cx: 5.0, cy: 6.0, dim: 0.5, radius_frac: 0.1, feather_frac: 0.1, alpha: 1.0,
                mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0 }), video: None };
        let u = build_fx_u(&st, 1000, 2000);
        assert_eq!(u.a, [1000.0, 2000.0, 1.0, 1.0]);     // ow, oh, style=ripple, 1 hit
        assert_eq!(u.b[3], 1.0);                          // spot active
        assert!((u.b[2] - 0.5).abs() < 1e-6);             // dim*alpha
        assert!((u.color[0] - 1.0).abs() < 1e-6 && u.color[1] < 1e-6); // red normalized
        assert_eq!(u.hits[0], [10.0, 20.0, 0.4, 0.0]);
    }
    #[test]
    fn no_spot_sets_inactive() {
        let st = FxState { style: ClickFxStyle::None, color: [0,0,0], intensity: 1.0, hits: vec![], spot: None, video: None };
        assert_eq!(build_fx_u(&st, 8, 8).b[3], 0.0);
    }
    #[test]
    fn spot_mode_id_and_tint_pack() {
        use crate::settings::model::SpotlightMode::*;
        assert_eq!(spot_mode_id(Classic), 0.0);
        assert_eq!(spot_mode_id(Nebula), 4.0);
        assert_eq!(spot_mode_id(Vignette), 5.0);
        let st = crate::export::fx_state::FxState { style: crate::settings::model::ClickFxStyle::None,
            color: [0,0,0], intensity: 1.0, hits: vec![],
            spot: Some(crate::export::fx_state::Spot { cx: 1.0, cy: 2.0, dim: 0.5, radius_frac: 0.1,
                feather_frac: 0.1, alpha: 1.0, mode: Nebula, tint: [255, 0, 128], t: 3.0 }), video: None };
        let u = build_fx_u(&st, 100, 100);
        assert_eq!(u.d[0], 4.0);                 // mode = nebula
        assert!((u.d[1] - 3.0).abs() < 1e-6);    // time
        assert!((u.tint[0] - 1.0).abs() < 1e-6 && u.tint[2] > 0.49); // tint r=1, b~0.5
    }
}
