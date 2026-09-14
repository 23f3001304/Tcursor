// The CPU stand-in for `fx_lens.wgsl`, used when no wgpu adapter is available (`select_fx`).
//
// What it mirrors: the MAGNIFICATION (`fx_lens::ZOOM` - the readable part of the glass), the drop
// shadow, the lift + cool cast, and the click ink drop / over-text ring. The shader re-samples the
// frame through a displaced UV; this path composites in place over the same buffer, so `magnify`
// first copies the shape's box aside and re-samples THAT - the undisturbed source the in-place
// draw would otherwise have lost (the trade `clickdraw.rs` still makes for Shockwave).
//
// DELIBERATE DIFFERENCES: no rim bend (`LENS_DISP`), no rim frost, no rim light. A glass cursor on
// a CPU-only machine is a clean magnifier without the edge glint; nothing moves or is placed
// differently, and the text under it reads exactly as it does on the GPU.
use crate::export::fx::fx_lens::{BackLens, CursorLens, Lenses, ZOOM};

/// The lift + cool cast inside a shape, per channel (RGB) - the shader's `(1.05, 1.08, 1.14)`.
const LIFT: [f32; 3] = [1.05, 1.08, 1.14];
/// The shadow strength and drop, mirroring `fx_lens.wgsl`'s `LENS_SHADOW` / `LENS_DROP`.
const SHADOW: f32 = 0.25;
const DROP: f32 = 2.0;
const INK: f32 = 0.30;

/// Blend RGB `c` over the BGRA pixel at byte index `i` (the `clickdraw::blend` primitive, kept
/// local so the two files' gains stay independently tunable).
fn over(out: &mut [u8], i: usize, c: [f32; 3], a: f32) {
    let a = a.clamp(0.0, 1.0);
    if a <= 0.002 || i + 3 >= out.len() { return; }
    for (k, ch) in [c[2], c[1], c[0]].into_iter().enumerate() {
        out[i + k] = (ch * a + out[i + k] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    }
}

/// Multiply the BGRA pixel at `i` down by `f` (0 = black, 1 = untouched) - the drop shadow.
fn darken(out: &mut [u8], i: usize, f: f32) {
    if i + 3 >= out.len() { return; }
    for k in 0..3 { out[i + k] = (out[i + k] as f32 * f.clamp(0.0, 1.0)) as u8; }
}

/// Visit every pixel of the axis-aligned box `(x0,y0)..(x1,y1)`, clipped to the frame.
fn for_box(ow: u32, oh: u32, b: [f32; 4], mut f: impl FnMut(usize, f32, f32)) {
    let x0 = (b[0].floor() as i32).max(0);
    let y0 = (b[1].floor() as i32).max(0);
    let x1 = (b[2].ceil() as i32).min(ow as i32);
    let y1 = (b[3].ceil() as i32).min(oh as i32);
    for y in y0..y1 {
        for x in x0..x1 { f(((y as u32 * ow + x as u32) * 4) as usize, x as f32 + 0.5, y as f32 + 0.5); }
    }
}

/// Signed distance to a rounded rect - `fx_lens.wgsl::lens_rr_sd`.
fn rr_sd(px: f32, py: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32 {
    let (cx, cy) = ((mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5);
    let (qx, qy) = ((px - cx).abs() - ((mx[0] - mn[0]) * 0.5 - r), (py - cy).abs() - ((mx[1] - mn[1]) * 0.5 - r));
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// The ink drop's coverage at distance `d` - `fx_lens.wgsl::lens_ink`'s geometry.
fn ink_cov(d: f32, p: f32, reach: f32) -> f32 {
    if p < 0.0 { return 0.0; }
    let r = (crate::export::fx::clickfx::ease_out(p) * reach).max(0.5);
    (1.0 - crate::export::fx::clickfx::smoothstep(r * 0.55, r, d)) * (1.0 - p.clamp(0.0, 1.0))
}

/// The un-shadowed, un-drawn frame under the box `b`, copied aside before anything in it changes -
/// the source `magnify` re-samples. `(x0, y0, w, h)` in whole pixels, clipped to the frame.
struct Source { x0: i32, y0: i32, w: i32, h: i32, px: Vec<u8> }

fn snapshot(out: &[u8], ow: u32, oh: u32, b: [f32; 4]) -> Source {
    let (x0, y0) = ((b[0].floor() as i32).max(0), (b[1].floor() as i32).max(0));
    let (x1, y1) = ((b[2].ceil() as i32).min(ow as i32), (b[3].ceil() as i32).min(oh as i32));
    let (w, h) = ((x1 - x0).max(0), (y1 - y0).max(0));
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y0 + h {
        let s = ((y as u32 * ow + x0 as u32) * 4) as usize;
        px.extend_from_slice(&out[s..s + (w * 4) as usize]);
    }
    Source { x0, y0, w, h, px }
}

/// The magnified, lifted colour (RGB) the glass shows at `(px, py)`: the snapshot sampled
/// (bilinear) at the point `1/ZOOM` of the way out from the shape's centre `c`, times `LIFT`.
fn magnify(src: &Source, c: [f32; 2], px: f32, py: f32) -> [f32; 3] {
    let (sx, sy) = (c[0] + (px - c[0]) / ZOOM - src.x0 as f32 - 0.5, c[1] + (py - c[1]) / ZOOM - src.y0 as f32 - 0.5);
    let (fx, fy) = (sx.floor(), sy.floor());
    let (tx, ty) = (sx - fx, sy - fy);
    let tap = |x: i32, y: i32| -> [f32; 3] {
        if src.w <= 0 || src.h <= 0 { return [0.0; 3]; }
        let (x, y) = (x.clamp(0, src.w - 1), y.clamp(0, src.h - 1));
        let i = ((y * src.w + x) * 4) as usize;
        if i + 2 >= src.px.len() { return [0.0; 3]; }
        [src.px[i + 2] as f32, src.px[i + 1] as f32, src.px[i] as f32]
    };
    let (a, b, d, e) = (tap(fx as i32, fy as i32), tap(fx as i32 + 1, fy as i32), tap(fx as i32, fy as i32 + 1), tap(fx as i32 + 1, fy as i32 + 1));
    let mut out = [0.0f32; 3];
    for k in 0..3 {
        let top = a[k] + (b[k] - a[k]) * tx;
        let bot = d[k] + (e[k] - d[k]) * tx;
        out[k] = ((top + (bot - top) * ty) * LIFT[k]).min(255.0);
    }
    out
}

/// The cursor back: shadow, the magnified frame, then its click look (a ring over text, an ink drop
/// otherwise).
fn draw_back(out: &mut [u8], ow: u32, oh: u32, b: &BackLens, accent: [u8; 3]) {
    let c = [(b.mn[0] + b.mx[0]) * 0.5, (b.mn[1] + b.mx[1]) * 0.5];
    let half = [(b.mx[0] - b.mn[0]) * 0.5 * b.squash, (b.mx[1] - b.mn[1]) * 0.5 * b.squash];
    let (mn, mx) = ([c[0] - half[0], c[1] - half[1]], [c[0] + half[0], c[1] + half[1]]);
    let r = b.r * b.squash;
    let reach = half[0].hypot(half[1]) * 1.2;
    let acc = [accent[0] as f32, accent[1] as f32, accent[2] as f32];
    let bx = [mn[0] - DROP - 2.0, mn[1] - 2.0, mx[0] + DROP + 2.0, mx[1] + DROP + 2.0];
    let src = snapshot(out, ow, oh, bx);
    for_box(ow, oh, bx, |i, px, py| {
        let sd = rr_sd(px, py, mn, mx, r);
        let m = (0.5 - sd).clamp(0.0, 1.0);
        let sh = (0.5 - rr_sd(px, py - DROP, mn, mx, r) / 1.5).clamp(0.0, 1.0);
        darken(out, i, 1.0 - SHADOW * sh * (1.0 - m));
        if m <= 0.004 { return; }
        over(out, i, magnify(&src, c, px, py), m);
        if b.ink < 0.0 { return; }
        if b.ring {
            let band = (1.0 - (sd + 1.5).abs() / 1.5).clamp(0.0, 1.0);
            over(out, i, acc, band * (1.0 - b.ink.clamp(0.0, 1.0)) * 0.8 * m);
        } else {
            over(out, i, acc, INK * ink_cov((px - b.ink_at[0]).hypot(py - b.ink_at[1]), b.ink, reach) * m);
        }
    });
}

/// The sprite lens: the pack's own alpha mask, shadowed and magnified. The busy rotation is undone
/// the same way `lens_cov` does it, so a spinning glass cursor's lens spins with it.
fn draw_glass(out: &mut [u8], ow: u32, oh: u32, l: &CursorLens, accent: [u8; 3]) {
    let half = [(l.cbox[2] * 0.5 * l.squash).max(0.5), (l.cbox[3] * 0.5 * l.squash).max(0.5)];
    let reach = half[0].hypot(half[1]) * 1.2;
    let acc = [accent[0] as f32, accent[1] as f32, accent[2] as f32];
    let span = half[0].hypot(half[1]);
    let cov = |px: f32, py: f32| -> f32 {
        let (dx, dy) = (px - l.cbox[0], py - l.cbox[1]);
        let (s, k) = ((-l.angle).sin(), (-l.angle).cos());
        let (ux, uy) = (dx * k - dy * s, dx * s + dy * k);
        let (u, v) = (ux / half[0] * 0.5 + 0.5, uy / half[1] * 0.5 + 0.5);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) { return 0.0; }
        let sx = ((u * l.mask.w as f32) as u32).min(l.mask.w.saturating_sub(1));
        let sy = ((v * l.mask.h as f32) as u32).min(l.mask.h.saturating_sub(1));
        l.mask.a.get((sy * l.mask.w + sx) as usize).map_or(0.0, |&a| a as f32 / 255.0)
    };
    let b = [l.cbox[0] - span - DROP - 2.0, l.cbox[1] - span - 2.0,
             l.cbox[0] + span + DROP + 2.0, l.cbox[1] + span + DROP + 2.0];
    let src = snapshot(out, ow, oh, b);
    let c = [l.cbox[0], l.cbox[1]];
    for_box(ow, oh, b, |i, px, py| {
        let m = cov(px, py);
        let sh = cov(px, py - DROP) * 0.4 + (cov(px - 1.5, py - DROP) + cov(px + 1.5, py - DROP)
            + cov(px, py - DROP + 1.5) + cov(px, py - DROP - 1.5)) * 0.15;
        darken(out, i, 1.0 - SHADOW * sh * (1.0 - m));
        if m <= 0.004 { return; }
        over(out, i, magnify(&src, c, px, py), m);
        over(out, i, acc, INK * ink_cov((px - l.ink_at[0]).hypot(py - l.ink_at[1]), l.ink, reach) * m);
    });
}

/// Both glass shapes, back first - the stacking order `fx_lens.wgsl::lens_fx` uses.
pub fn draw_lens(out: &mut [u8], ow: u32, oh: u32, l: &Lenses, accent: [u8; 3]) {
    if let Some(b) = &l.back { draw_back(out, ow, oh, b, accent); }
    if let Some(g) = &l.glass { draw_glass(out, ow, oh, g, accent); }
}

#[cfg(test)]
#[path = "fx_lensdraw_tests.rs"]
mod tests;
