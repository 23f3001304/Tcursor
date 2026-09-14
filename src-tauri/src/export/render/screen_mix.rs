//! The display-switch cross-dissolve, done on the DECODED SCREEN BUFFER rather than inside the
//! compositors.
//!
//! At a switch the panel eases from the old display's aspect to the new one's while the two
//! pictures cross-fade. Both compositors already draw exactly one screen source cropped to one
//! rect, so the cheapest way to keep CPU and GPU (and therefore preview and export) on ONE code
//! path is to hand them a single nv12 frame that is already the blend: the held frame's own span
//! rect is resampled into the new span's rect and mixed in at `1 - alpha`. Drawing that rect into
//! the eased panel then squashes BOTH pictures by the same factor, which is what makes the morph
//! read as one picture changing shape instead of two pictures sliding over each other.
//!
//! Everything happens in nv12, the format the decoders and both compositors already agree on, so
//! no conversion is added to the frame path. The work only ever runs during a 350 ms transition.

use crate::export::types::RectF;

/// Blend the held `prev` frame into `cur` over the destination rect, writing the result to `out`
/// (nv12, `sw` x `sh`). `alpha` is the weight of `cur` - 0 at the switch instant, 1 at the end of
/// the transition. `prev_src`/`dst` are the two spans' source rects. A degenerate alpha (already
/// all one picture) still copies, so the caller always gets a complete frame back.
pub fn blend_into(out: &mut Vec<u8>, cur: &[u8], prev: &[u8], sw: u32, sh: u32,
                  prev_src: RectF, dst: RectF, alpha: f32) {
    out.clear();
    out.extend_from_slice(cur);
    let a = alpha.clamp(0.0, 1.0);
    if a >= 0.999 || cur.len() != prev.len() { return; }
    let y_len = (sw as usize) * (sh as usize);
    if out.len() < y_len + y_len / 2 { return; }
    let (y_out, uv_out) = out.split_at_mut(y_len);
    // Y at full resolution, then the interleaved half-res UV plane - the same rects halved.
    plane(y_out, &prev[..y_len], sw, sh, prev_src, dst, a, 1);
    plane(uv_out, &prev[y_len..], sw / 2, sh / 2, half(prev_src), half(dst), a, 2);
}

/// A rect in half-resolution (chroma) coordinates.
fn half(r: RectF) -> RectF { RectF { x: r.x / 2.0, y: r.y / 2.0, w: r.w / 2.0, h: r.h / 2.0 } }

/// Blend one plane: for every pixel of `dst`, bilinearly sample `prev` at the matching point of
/// `prev_src` and mix it under the destination pixel at `1 - a`. `bpp` is 1 (Y) or 2 (UV).
fn plane(out: &mut [u8], prev: &[u8], w: u32, h: u32, prev_src: RectF, dst: RectF, a: f32, bpp: usize) {
    let (dw, dh) = (dst.w.max(1.0), dst.h.max(1.0));
    let (x0, y0) = (dst.x.max(0.0) as u32, dst.y.max(0.0) as u32);
    let (x1, y1) = (((dst.x + dw) as u32).min(w), ((dst.y + dh) as u32).min(h));
    let (fx, fy) = (prev_src.w.max(1.0) / dw, prev_src.h.max(1.0) / dh);
    let row = w as usize * bpp;
    for y in y0..y1 {
        let sy = prev_src.y + (y as f32 + 0.5 - dst.y) * fy - 0.5;
        for x in x0..x1 {
            let sx = prev_src.x + (x as f32 + 0.5 - dst.x) * fx - 0.5;
            let di = y as usize * row + x as usize * bpp;
            for c in 0..bpp {
                let s = sample(prev, w, h, sx, sy, c, bpp);
                out[di + c] = (out[di + c] as f32 * a + s * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Bilinear sample of channel `c` at (`sx`, `sy`), clamped to the plane's edges.
fn sample(p: &[u8], w: u32, h: u32, sx: f32, sy: f32, c: usize, bpp: usize) -> f32 {
    let (xf, yf) = (sx.clamp(0.0, (w - 1) as f32), sy.clamp(0.0, (h - 1) as f32));
    let (xi, yi) = (xf.floor() as u32, yf.floor() as u32);
    let (x2, y2) = ((xi + 1).min(w - 1), (yi + 1).min(h - 1));
    let (tx, ty) = (xf - xi as f32, yf - yi as f32);
    let at = |x: u32, y: u32| p[y as usize * w as usize * bpp + x as usize * bpp + c] as f32;
    let top = at(xi, yi) * (1.0 - tx) + at(x2, yi) * tx;
    let bot = at(xi, y2) * (1.0 - tx) + at(x2, y2) * tx;
    top * (1.0 - ty) + bot * ty
}

#[cfg(test)]
#[path = "screen_mix_tests.rs"]
mod tests;
