use crate::export::fx::fx_state::FxState;
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
    pub d: [f32; 4],                // spot_mode_id, time_s, keep_camera_lit(0/1), cam_radius(px)
    pub tint: [f32; 4],             // r, g, b (0..1), _pad
    pub color: [f32; 4],            // r, g, b (0..1), _pad
    pub color2: [f32; 4],           // `color` rotated NEON_HUE_SHIFT of hue (Neon's 2nd tube), _pad
    pub hits: [[f32; 4]; MAX_HITS], // x, y, progress, _pad
    pub e: [f32; 4],                // video_mode_id, alpha, t, _pad
    pub cam: [f32; 4],              // camera-exclusion rect (px): min_x, min_y, max_x, max_y
    // The glass cursor material (`fx_lens.wgsl`). `lens_b[1]`/`back_b[1]` are the on/off slots.
    pub lens_a: [f32; 4],           // sprite lens box (px): centre x, centre y, w, h
    pub lens_b: [f32; 4],           // busy angle (rad), on, click squash, ink progress (<0 = none)
    pub lens_c: [f32; 4],           // ink origin (px): x, y, _pad, _pad
    pub back_a: [f32; 4],           // cursor-back rounded rect (px): min_x, min_y, max_x, max_y
    pub back_b: [f32; 4],           // corner radius (px), on, click squash, ink progress
    pub back_c: [f32; 4],           // ink origin (px): x, y, ring-instead-of-drop, _pad
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

/// Degrees of hue Neon's second tube is rotated from the user's tint. *Why in Rust and not in the
/// shader:* an RGB->HSV->RGB round trip is a dozen lines of branchy WGSL run per fragment for a
/// value that changes once per frame; here it is one `build_fx_u` call, and `clickdraw.rs` gets
/// the same number for free instead of porting the shader version.
pub const NEON_HUE_SHIFT: f32 = 30.0;

/// Rotate `rgb` by `deg` degrees of hue, keeping saturation and value. Returns 0..1 components.
pub fn hue_shift(rgb: [u8; 3], deg: f32) -> [f32; 3] {
    let (r, g, b) = (rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0);
    let (v, mn) = (r.max(g).max(b), r.min(g).min(b));
    let c = v - mn;
    let s = if v <= 0.0 { 0.0 } else { c / v };
    let h6 = if c <= 0.0 { 0.0 }
        else if v == r { ((g - b) / c).rem_euclid(6.0) }
        else if v == g { (b - r) / c + 2.0 }
        else { (r - g) / c + 4.0 };
    let h = ((h6 * 60.0 + deg).rem_euclid(360.0)) / 60.0;
    let f = h - h.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match h.floor() as i32 % 6 {
        0 => [v, t, p], 1 => [q, v, p], 2 => [p, v, t],
        3 => [p, q, v], 4 => [t, p, v], _ => [v, p, q],
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
    let (d, tint, cam) = match state.spot {
        Some(s) => ([spot_mode_id(s.mode), s.t, if s.dim_camera { 0.0 } else { 1.0 }, s.cam_radius],
                    [s.tint[0] as f32 / 255.0, s.tint[1] as f32 / 255.0, s.tint[2] as f32 / 255.0, 0.0],
                    s.cam_rect),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    let e = match state.video {
        Some(v) => [video_mode_id(v.mode), v.alpha, v.t, 0.0],
        None => [0.0; 4],
    };
    let c2 = hue_shift(state.color, NEON_HUE_SHIFT);
    let (lens_a, lens_b, lens_c) = match state.lens.as_ref().and_then(|l| l.glass.as_ref()) {
        Some(g) => (g.cbox, [g.angle, 1.0, g.squash, g.ink], [g.ink_at[0], g.ink_at[1], 0.0, 0.0]),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    let (back_a, back_b, back_c) = match state.lens.as_ref().and_then(|l| l.back.as_ref()) {
        Some(b) => ([b.mn[0], b.mn[1], b.mx[0], b.mx[1]], [b.r, 1.0, b.squash, b.ink],
                    [b.ink_at[0], b.ink_at[1], if b.ring { 1.0 } else { 0.0 }, 0.0]),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    FxU {
        a: [ow as f32, oh as f32, style, n], b, c, d, tint,
        color: [state.color[0] as f32 / 255.0, state.color[1] as f32 / 255.0, state.color[2] as f32 / 255.0, 0.0],
        color2: [c2[0], c2[1], c2[2], 0.0],
        hits, e, cam, lens_a, lens_b, lens_c, back_a, back_b, back_c,
    }
}

#[cfg(test)]
#[path = "fx_uniforms_tests.rs"]
mod tests;
