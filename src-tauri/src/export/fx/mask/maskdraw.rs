use crate::export::fx::mask::fx_masks::MaskDraw;
use crate::export::fx::mask::rrect_sd;

pub fn blur_sigma(amount_px: f32) -> f32 {
    let r = amount_px.round().max(1.0);
    let k = 2.0 * r + 1.0;
    ((k * k - 1.0) / 4.0).sqrt()
}

pub fn draw_masks(out: &mut [u8], ow: u32, oh: u32, masks: &[MaskDraw]) {
    if masks.is_empty() {
        return;
    }
    let base = out.to_vec();
    for m in masks {
        if m.alpha <= 0.0 {
            continue;
        }
        match m.kind {
            1 => blur(out, &base, ow, oh, m),
            2 => pixelate(out, &base, ow, oh, m),
            3 => highlight(out, ow, oh, m),
            _ => {}
        }
    }
}

fn cov(m: &MaskDraw, x: u32, y: u32) -> f32 {
    let sd = rrect_sd(x as f32 + 0.5, y as f32 + 0.5, m.mn, m.mx, m.r);
    (0.5 - sd / m.feather_px.max(1.0)).clamp(0.0, 1.0) * m.alpha
}

fn bounds(m: &MaskDraw, ow: u32, oh: u32) -> (u32, u32, u32, u32) {
    let pad = m.feather_px.max(1.0) + 1.0;
    let x0 = (m.mn[0] - pad).floor().max(0.0) as u32;
    let y0 = (m.mn[1] - pad).floor().max(0.0) as u32;
    let x1 = ((m.mx[0] + pad).ceil().max(0.0) as u32).min(ow);
    let y1 = ((m.mx[1] + pad).ceil().max(0.0) as u32).min(oh);
    (x0.min(ow), y0.min(oh), x1, y1)
}

fn mix_in(out: &mut [u8], i: usize, src: [f32; 3], k: f32) {
    for c in 0..3 {
        out[i + c] = (out[i + c] as f32 + (src[c] - out[i + c] as f32) * k).round() as u8;
    }
}

fn blur(out: &mut [u8], base: &[u8], ow: u32, oh: u32, m: &MaskDraw) {
    let (x0, y0, x1, y1) = bounds(m, ow, oh);
    let (bw, bh) = (
        (x1.saturating_sub(x0)) as usize,
        (y1.saturating_sub(y0)) as usize,
    );
    if bw == 0 || bh == 0 {
        return;
    }
    let mut a = vec![0.0f32; bw * bh * 3];
    for y in 0..bh {
        for x in 0..bw {
            let i = (((y0 + y as u32) * ow + x0 + x as u32) * 4) as usize;
            let o = (y * bw + x) * 3;
            a[o] = base[i] as f32;
            a[o + 1] = base[i + 1] as f32;
            a[o + 2] = base[i + 2] as f32;
        }
    }
    let mut b = vec![0.0f32; bw * bh * 3];
    let r = m.amount_px.round().max(1.0) as usize;
    for _ in 0..3 {
        box_pass(&a, &mut b, bw, bh, r, false);
        box_pass(&b, &mut a, bw, bh, r, true);
    }
    for y in 0..bh {
        for x in 0..bw {
            let (gx, gy) = (x0 + x as u32, y0 + y as u32);
            let k = cov(m, gx, gy);
            if k <= 0.0 {
                continue;
            }
            let o = (y * bw + x) * 3;
            mix_in(
                out,
                ((gy * ow + gx) * 4) as usize,
                [a[o], a[o + 1], a[o + 2]],
                k,
            );
        }
    }
}

fn box_pass(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize, vertical: bool) {
    let (n, m, sa, sb) = if vertical {
        (w, h, 3, w * 3)
    } else {
        (h, w, w * 3, 3)
    };
    for a in 0..n {
        for c in 0..3 {
            let at = |b: usize| a * sa + b * sb + c;
            let mut sum = 0.0f32;
            let mut cnt = 0.0f32;
            for b in 0..(r + 1).min(m) {
                sum += src[at(b)];
                cnt += 1.0;
            }
            for b in 0..m {
                dst[at(b)] = sum / cnt.max(1.0);
                if b + r + 1 < m {
                    sum += src[at(b + r + 1)];
                    cnt += 1.0;
                }
                if b >= r {
                    sum -= src[at(b - r)];
                    cnt -= 1.0;
                }
            }
        }
    }
}

fn pixelate(out: &mut [u8], base: &[u8], ow: u32, oh: u32, m: &MaskDraw) {
    let (x0, y0, x1, y1) = bounds(m, ow, oh);
    let cell = m.amount_px.max(2.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let k = cov(m, x, y);
            if k <= 0.0 {
                continue;
            }
            let qx = ((x as f32 + 0.5 - m.mn[0]) / cell).floor() * cell + m.mn[0] + cell * 0.5;
            let qy = ((y as f32 + 0.5 - m.mn[1]) / cell).floor() * cell + m.mn[1] + cell * 0.5;
            let sx = (qx.floor().max(0.0) as u32).min(ow - 1);
            let sy = (qy.floor().max(0.0) as u32).min(oh - 1);
            let s = ((sy * ow + sx) * 4) as usize;
            mix_in(
                out,
                ((y * ow + x) * 4) as usize,
                [base[s] as f32, base[s + 1] as f32, base[s + 2] as f32],
                k,
            );
        }
    }
}

fn highlight(out: &mut [u8], ow: u32, oh: u32, m: &MaskDraw) {
    let d = (m.dim.clamp(0.0, 1.0) * m.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if d <= 0.0 {
        return;
    }
    for y in 0..oh {
        for x in 0..ow {
            let k = 1.0 - d * (1.0 - cov(m, x, y));
            if k >= 1.0 {
                continue;
            }
            let i = ((y * ow + x) * 4) as usize;
            for c in 0..3 {
                out[i + c] = (out[i + c] as f32 * k).round() as u8;
            }
        }
    }
}

#[cfg(test)]
#[path = "maskdraw_tests.rs"]
mod tests;
