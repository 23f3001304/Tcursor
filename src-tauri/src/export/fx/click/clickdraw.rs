use crate::export::fx::click::clickfx::{ease_out, fade_alpha, ripple_radius, smoothstep};
use crate::export::fx::fx_state::FxState;
use crate::export::fx::fx_uniforms::{hue_shift, NEON_HUE_SHIFT};
use crate::settings::model::ClickFxStyle;

const WHITE: [u8; 3] = [255, 255, 255];

fn blend(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    out[i] = (c[2] as f32 * a + out[i] as f32 * (1.0 - a)).round() as u8;
    out[i + 1] = (c[1] as f32 * a + out[i + 1] as f32 * (1.0 - a)).round() as u8;
    out[i + 2] = (c[0] as f32 * a + out[i + 2] as f32 * (1.0 - a)).round() as u8;
}

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

fn for_seg(
    ow: u32,
    oh: u32,
    a: (f32, f32),
    b: (f32, f32),
    rad: f32,
    mut f: impl FnMut(usize, f32),
) {
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let len2 = (bx * bx + by * by).max(1e-4);
    let half = len2.sqrt() * 0.5;
    for_disc(
        ow,
        oh,
        (a.0 + b.0) * 0.5,
        (a.1 + b.1) * 0.5,
        half + rad + 1.0,
        |i, _| {
            let px = ((i / 4) % ow as usize) as f32;
            let py = ((i / 4) / ow as usize) as f32;
            let t = (((px - a.0) * bx + (py - a.1) * by) / len2).clamp(0.0, 1.0);
            let (dx, dy) = (px - (a.0 + bx * t), py - (a.1 + by * t));
            f(i, (dx * dx + dy * dy).sqrt());
        },
    );
}

fn add_blend(out: &mut [u8], i: usize, c: [u8; 3], a: f32) {
    let a = a.max(0.0);
    if a <= 0.0 {
        return;
    }
    out[i] = (out[i] as f32 + c[2] as f32 * a).min(255.0) as u8;
    out[i + 1] = (out[i + 1] as f32 + c[1] as f32 * a).min(255.0) as u8;
    out[i + 2] = (out[i + 2] as f32 + c[0] as f32 * a).min(255.0) as u8;
}

fn glow(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, r: f32, c: [u8; 3], a: f32) {
    let r = r.max(1.0);
    for_disc(ow, oh, cx, cy, r * 2.0, |i, d| {
        add_blend(out, i, c, a * (-(d * d) / (r * r)).exp())
    });
}

fn flash(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, p: f32, inten: f32) {
    let g = (1.0 - smoothstep(0.0, 0.14, p)) * inten;
    if g > 0.0 {
        glow(out, ow, oh, cx, cy, oh as f32 * 0.015, WHITE, g);
    }
}

fn hash1(x: f32) -> f32 {
    ((x * 127.1).sin() * 43758.5453).fract()
}

fn particles(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, prog: f32, c: [u8; 3], a: f32) {
    let p = prog.clamp(0.0, 1.0);
    let (h, r) = (oh as f32, oh as f32 * 0.004);
    let g = a * (1.0 - p * p);
    for k in 0..14u32 {
        let ang = hash1(k as f32) * std::f32::consts::TAU;
        let sp = (0.4 + hash1(k as f32 + 7.0)) * h * 0.10;
        let (dx, dy) = (ang.cos() * sp, ang.sin() * sp);
        let pos = (cx + dx * p, cy + dy * p + h * 0.06 * p * p);
        let vel = (dx * 0.06, (dy + h * 0.12 * p) * 0.06);
        for_seg(
            ow,
            oh,
            (pos.0 - vel.0, pos.1 - vel.1),
            pos,
            r.max(1.0),
            |i, d| {
                let cov = (r - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    add_blend(out, i, c, g * cov);
                }
            },
        );
    }
}

fn ring_add(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    thick: f32,
    c: [u8; 3],
    a: f32,
) {
    let t = thick.max(1.0);
    for_disc(ow, oh, cx, cy, radius + t, |i, d| {
        let cov = (t - (d - radius).abs()).clamp(0.0, t) / t;
        if cov > 0.0 {
            add_blend(out, i, c, a * cov);
        }
    });
}

fn ring(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    thick: f32,
    c: [u8; 3],
    a: f32,
) {
    let t = thick.max(1.0);
    for_disc(ow, oh, cx, cy, radius + t, |i, d| {
        let cov = (t - (d - radius).abs()).clamp(0.0, t) / t;
        if cov > 0.0 {
            blend(out, i, c, a * cov);
        }
    });
}

fn soft_disc(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32) {
    for_disc(ow, oh, cx, cy, radius, |i, d| {
        let cov = 1.0 - smoothstep(radius * 0.2, radius, d);
        if cov > 0.0 {
            blend(out, i, c, a * cov);
        }
    });
}

fn disc_add(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32) {
    for_disc(ow, oh, cx, cy, radius + 1.0, |i, d| {
        let cov = (radius - d).clamp(0.0, 1.0);
        if cov > 0.0 {
            add_blend(out, i, c, a * cov);
        }
    });
}

fn u8c(v: [f32; 3]) -> [u8; 3] {
    [
        (v[0] * 255.0) as u8,
        (v[1] * 255.0) as u8,
        (v[2] * 255.0) as u8,
    ]
}

pub fn draw_clicks(out: &mut [u8], ow: u32, oh: u32, state: &FxState) {
    if matches!(state.style, ClickFxStyle::None) {
        return;
    }
    let h = oh as f32;
    let inten = state.intensity.clamp(0.0, 1.0);
    let c = state.color;
    for hit in &state.hits {
        let p = hit.progress.clamp(0.0, 1.0);
        let a = fade_alpha(p, state.intensity);
        let (x, y) = (hit.x, hit.y);
        flash(out, ow, oh, x, y, p, inten);
        match state.style {
            ClickFxStyle::Ripple => {
                let r1 = ripple_radius(p, h * 0.06);
                glow(out, ow, oh, x, y, r1 * 2.0, c, a * 0.15);
                ring(out, ow, oh, x, y, r1, h * 0.006, c, a);
                if p > 0.15 {
                    ring(
                        out,
                        ow,
                        oh,
                        x,
                        y,
                        ripple_radius(p - 0.15, h * 0.06),
                        h * 0.0045,
                        c,
                        a * 0.7,
                    );
                }
                if p > 0.30 {
                    ring(
                        out,
                        ow,
                        oh,
                        x,
                        y,
                        ripple_radius(p - 0.30, h * 0.06),
                        h * 0.003,
                        c,
                        a * 0.45,
                    );
                }
            }
            ClickFxStyle::Pulse => {
                let r = h * (0.01 + 0.025 * ease_out(p));
                soft_disc(out, ow, oh, x, y, r, c, a);
                ring_add(out, ow, oh, x, y, r, h * 0.004, c, a * 0.6);
                disc_add(
                    out,
                    ow,
                    oh,
                    x,
                    y,
                    h * 0.006,
                    WHITE,
                    a * (1.0 - smoothstep(0.0, 0.3, p)),
                );
            }
            ClickFxStyle::Glow => {
                let r = h
                    * 0.05
                    * (0.6 + 0.8 * p)
                    * (1.0 + 0.06 * (p * std::f32::consts::TAU * 2.0).sin());
                glow(out, ow, oh, x, y, r, c, a);
                glow(out, ow, oh, x, y, h * 0.008, WHITE, a * 0.5);
            }
            ClickFxStyle::Neon => {
                let r1 = ease_out(p) * h * 0.07;
                ring_add(out, ow, oh, x, y, r1, h * 0.01, c, a * 1.4);
                ring_add(out, ow, oh, x, y, r1, h * 0.01, WHITE, a * 0.25);
                if p > 0.12 {
                    let r2 = ease_out(p - 0.12) * h * 0.07;
                    ring_add(
                        out,
                        ow,
                        oh,
                        x,
                        y,
                        r2,
                        h * 0.008,
                        u8c(hue_shift(c, NEON_HUE_SHIFT)),
                        a,
                    );
                    ring_add(out, ow, oh, x, y, r2, h * 0.008, WHITE, a * 0.18);
                }
            }
            ClickFxStyle::Shockwave => {
                let r = ease_out(p) * h * 0.09;
                ring_add(out, ow, oh, x, y, r, h * 0.008, c, a * 0.5);
                ring_add(out, ow, oh, x, y, r + h * 0.004, h * 0.004, WHITE, a * 0.35);
            }
            ClickFxStyle::Particles => particles(out, ow, oh, x, y, p, c, a),
            ClickFxStyle::None => {}
        }
    }
}
