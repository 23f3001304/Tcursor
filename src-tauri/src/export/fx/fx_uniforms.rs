use crate::export::fx::fx_state::FxState;
use crate::settings::model::{ClickFxStyle, SpotlightMode, VideoFxMode};

pub const MAX_HITS: usize = 16;
pub const MAX_MASKS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FxU {
    pub a: [f32; 4],
    pub b: [f32; 4],
    pub c: [f32; 4],
    pub d: [f32; 4],
    pub tint: [f32; 4],
    pub color: [f32; 4],
    pub color2: [f32; 4],
    pub hits: [[f32; 4]; MAX_HITS],
    pub e: [f32; 4],
    pub cam: [f32; 4],
    pub lens_a: [f32; 4],
    pub lens_b: [f32; 4],
    pub lens_c: [f32; 4],
    pub back_a: [f32; 4],
    pub back_b: [f32; 4],
    pub back_c: [f32; 4],
    pub mask: [[f32; 4]; 3 * MAX_MASKS],
    pub grade: [[f32; 4]; 6],
}

pub fn style_id(s: ClickFxStyle) -> f32 {
    match s {
        ClickFxStyle::None => 0.0,
        ClickFxStyle::Ripple => 1.0,
        ClickFxStyle::Pulse => 2.0,
        ClickFxStyle::Glow => 3.0,
        ClickFxStyle::Shockwave => 4.0,
        ClickFxStyle::Particles => 5.0,
        ClickFxStyle::Neon => 6.0,
    }
}

pub const NEON_HUE_SHIFT: f32 = 30.0;

pub fn hue_shift(rgb: [u8; 3], deg: f32) -> [f32; 3] {
    let (r, g, b) = (
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    );
    let (v, mn) = (r.max(g).max(b), r.min(g).min(b));
    let c = v - mn;
    let s = if v <= 0.0 { 0.0 } else { c / v };
    let h6 = if c <= 0.0 {
        0.0
    } else if v == r {
        ((g - b) / c).rem_euclid(6.0)
    } else if v == g {
        (b - r) / c + 2.0
    } else {
        (r - g) / c + 4.0
    };
    let h = ((h6 * 60.0 + deg).rem_euclid(360.0)) / 60.0;
    let f = h - h.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match h.floor() as i32 % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

pub fn video_mode_id(m: VideoFxMode) -> f32 {
    match m {
        VideoFxMode::NebulaWash => 0.0,
        VideoFxMode::CinematicDim => 1.0,
        VideoFxMode::ScreenFocus => 2.0,
        VideoFxMode::ColorPop => 3.0,
    }
}

pub fn spot_mode_id(m: SpotlightMode) -> f32 {
    match m {
        SpotlightMode::Classic => 0.0,
        SpotlightMode::Blur => 1.0,
        SpotlightMode::Halo => 2.0,
        SpotlightMode::Breathing => 3.0,
        SpotlightMode::Nebula => 4.0,
        SpotlightMode::Vignette => 5.0,
    }
}

pub fn pack_masks(masks: &[crate::export::fx::fx_masks::MaskDraw]) -> [[f32; 4]; 3 * MAX_MASKS] {
    let mut m = [[0.0f32; 4]; 3 * MAX_MASKS];
    for (i, d) in masks.iter().take(MAX_MASKS).enumerate() {
        m[i * 3] = [d.mn[0], d.mn[1], d.mx[0], d.mx[1]];
        m[i * 3 + 1] = [d.r, d.feather_px, d.amount_px, d.kind as f32];
        m[i * 3 + 2] = [d.dim, d.alpha, 0.0, 0.0];
    }
    m
}

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
            (
                [
                    s.cx,
                    s.cy,
                    s.dim.clamp(0.0, 1.0) * s.alpha.clamp(0.0, 1.0),
                    1.0,
                ],
                [r_in, r_out, state.intensity, 0.0],
            )
        }
        None => ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, state.intensity, 0.0]),
    };
    let (d, tint, cam) = match state.spot {
        Some(s) => (
            [
                spot_mode_id(s.mode),
                s.t,
                if s.dim_camera { 0.0 } else { 1.0 },
                s.cam_radius,
            ],
            [
                s.tint[0] as f32 / 255.0,
                s.tint[1] as f32 / 255.0,
                s.tint[2] as f32 / 255.0,
                0.0,
            ],
            s.cam_rect,
        ),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    let e = match state.video {
        Some(v) => [video_mode_id(v.mode), v.alpha, v.t, 0.0],
        None => [0.0; 4],
    };
    let c2 = hue_shift(state.color, NEON_HUE_SHIFT);
    let (lens_a, lens_b, lens_c) = match state.lens.as_ref().and_then(|l| l.glass.as_ref()) {
        Some(g) => (
            g.cbox,
            [g.angle, 1.0, g.squash, g.ink],
            [g.ink_at[0], g.ink_at[1], 0.0, 0.0],
        ),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    let (back_a, back_b, back_c) = match state.lens.as_ref().and_then(|l| l.back.as_ref()) {
        Some(b) => (
            [b.mn[0], b.mn[1], b.mx[0], b.mx[1]],
            [b.r, 1.0, b.squash, b.ink],
            [
                b.ink_at[0],
                b.ink_at[1],
                if b.ring { 1.0 } else { 0.0 },
                0.0,
            ],
        ),
        None => ([0.0; 4], [0.0; 4], [0.0; 4]),
    };
    FxU {
        a: [ow as f32, oh as f32, style, n],
        b,
        c,
        d,
        tint,
        color: [
            state.color[0] as f32 / 255.0,
            state.color[1] as f32 / 255.0,
            state.color[2] as f32 / 255.0,
            0.0,
        ],
        color2: [c2[0], c2[1], c2[2], 0.0],
        hits,
        e,
        cam,
        lens_a,
        lens_b,
        lens_c,
        back_a,
        back_b,
        back_c,
        mask: pack_masks(&state.masks),
        grade: pack_grade(state.grade.as_ref()),
    }
}

pub fn pack_grade(g: Option<&crate::export::grade::GradeParams>) -> [[f32; 4]; 6] {
    match g {
        None => [[0.0; 4]; 6],
        Some(p) => [
            [p.exposure, p.contrast, p.saturation, p.vignette],
            [p.temp, p.tint, 1.0, 0.0],
            [p.lift[0], p.lift[1], p.lift[2], 0.0],
            [p.gamma[0], p.gamma[1], p.gamma[2], 0.0],
            [p.gain[0], p.gain[1], p.gain[2], 0.0],
            [0.0; 4],
        ],
    }
}

#[cfg(test)]
#[path = "fx_uniforms_tests.rs"]
mod tests;
