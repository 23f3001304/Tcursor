use crate::export::fx::clickfx::{fade_alpha, ripple_radius};
use crate::export::fx::fx_state::FxState;
use crate::settings::model::ClickFxStyle;

/// Alpha-blend RGB `c` over the BGRA pixel at byte index `i`.
fn blend(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 { return; }
    out[i] = (c[2] as f32 * a + out[i] as f32 * (1.0 - a)).round() as u8;         // B
    out[i + 1] = (c[1] as f32 * a + out[i + 1] as f32 * (1.0 - a)).round() as u8; // G
    out[i + 2] = (c[0] as f32 * a + out[i + 2] as f32 * (1.0 - a)).round() as u8; // R
}

/// Visit every pixel within `rad` of `(cx,cy)` with its distance, clipped to frame.
fn for_disc(ow: u32, oh: u32, cx: f32, cy: f32, rad: f32, mut f: impl FnMut(usize, f32)) {
    let r = rad.ceil() as i32 + 1;
    let (cxi, cyi) = (cx.round() as i32, cy.round() as i32);
    for y in (cyi - r).max(0)..(cyi + r).min(oh as i32) {
        for x in (cxi - r).max(0)..(cxi + r).min(ow as i32) {
            let d = (((x as f32 - cx).powi(2)) + ((y as f32 - cy).powi(2))).sqrt();
            f(((y as u32 * ow + x as u32) * 4) as usize, d);
        }
    }
}

/// Additive blend of RGB `c` (scaled by `a`) onto the BGRA pixel at byte `i`, clamped.
fn add_blend(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 { return; }
    out[i] = (out[i] as f32 + c[2] as f32 * a).min(255.0) as u8;         // B
    out[i + 1] = (out[i + 1] as f32 + c[1] as f32 * a).min(255.0) as u8; // G
    out[i + 2] = (out[i + 2] as f32 + c[0] as f32 * a).min(255.0) as u8; // R
}

/// Soft additive bloom: gaussian falloff to radius `r`.
fn glow(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, r: f32, c: [u8; 3], a: f32) {
    let r = r.max(1.0);
    for_disc(ow, oh, cx, cy, r * 2.0, |i, d| add_blend(out, i, c, a * (-(d * d) / (r * r)).exp()));
}

/// A burst of 12 additive sparks flung out from (cx,cy), arcing down with progress.
fn particles(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, prog: f32, c: [u8; 3], a: f32) {
    let spread = oh as f32 * 0.10;
    for k in 0..12u32 {
        let ang = k as f32 / 12.0 * std::f32::consts::TAU;
        let px = cx + ang.cos() * spread * prog;
        let py = cy + ang.sin() * spread * prog + oh as f32 * 0.06 * prog * prog;
        let r = oh as f32 * 0.004 + 1.0;
        for_disc(ow, oh, px, py, r, |i, d| {
            let cov = (r - d).clamp(0.0, 1.0);
            if cov > 0.0 { add_blend(out, i, c, a * cov); }
        });
    }
}

/// Bright additive ring (neon / shockwave fallback).
fn ring_add(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, thick: f32, c: [u8; 3], a: f32) {
    let t = thick.max(1.0);
    for_disc(ow, oh, cx, cy, radius + t, |i, d| {
        let cov = (t - (d - radius).abs()).clamp(0.0, t) / t;
        if cov > 0.0 { add_blend(out, i, c, a * cov); }
    });
}

fn ring(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, thick: f32, c: [u8; 3], a: f32) {
    let t = thick.max(1.0);
    for_disc(ow, oh, cx, cy, radius + t, |i, d| {
        let cov = (t - (d - radius).abs()).clamp(0.0, t) / t; // AA band falloff
        if cov > 0.0 { blend(out, i, c, a * cov); }
    });
}

fn disc(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32) {
    for_disc(ow, oh, cx, cy, radius, |i, d| {
        let cov = (radius - d).clamp(0.0, 1.0); // 1px AA edge
        if cov > 0.0 { blend(out, i, c, a * cov); }
    });
}

/// Draw the click effects (ripple/pulse/glow/shockwave/particles/neon) for `state`.
pub fn draw_clicks(out: &mut [u8], ow: u32, oh: u32, state: &FxState) {
    if matches!(state.style, ClickFxStyle::None) { return; }
    let r_max = oh as f32 * 0.06;
    for h in &state.hits {
        let a = fade_alpha(h.progress, state.intensity);
        match state.style {
            ClickFxStyle::Ripple => ring(out, ow, oh, h.x, h.y, ripple_radius(h.progress, r_max), oh as f32 * 0.006, state.color, a),
            ClickFxStyle::Pulse => disc(out, ow, oh, h.x, h.y, oh as f32 * 0.02, state.color, a),
            ClickFxStyle::Glow => glow(out, ow, oh, h.x, h.y, oh as f32 * 0.05 * (0.6 + 0.8 * h.progress), state.color, a),
            ClickFxStyle::Neon => ring_add(out, ow, oh, h.x, h.y, h.progress * oh as f32 * 0.07, oh as f32 * 0.01, state.color, (a * 1.4).min(1.0)),
            ClickFxStyle::Shockwave => ring_add(out, ow, oh, h.x, h.y, h.progress * oh as f32 * 0.09, oh as f32 * 0.008, state.color, (a * 0.7).min(1.0)),
            ClickFxStyle::Particles => particles(out, ow, oh, h.x, h.y, h.progress, state.color, a),
            ClickFxStyle::None => {}
        }
    }
}
