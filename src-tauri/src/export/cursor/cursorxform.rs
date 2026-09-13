// Rotated/scaled cursor blit, for pack v2's animated busy state. `cursordraw`'s own `blit` is a
// nearest-neighbour axis-aligned copy and stays the fast path for every still cursor - this is
// only reached when `busy_pose` asks for a real transform, so the extra cost is confined to the
// one animated sprite.
//
// Destination-driven: each output pixel is inverse-mapped back into sprite space and bilinearly
// sampled. Sampling forward (walking the source) would leave seams at any angle.
use crate::export::cursor::cursordraw::CursorSprite;

/// Alpha-blit `spr` rotated `angle_deg` CLOCKWISE and scaled by `extra` about its hotspot, with
/// the hotspot itself landing on `anchor` (output px) - the same anchor the untransformed blit
/// uses, so an animating busy cursor never drifts off the cursor point.
///
/// `scale` is the sprite's own output scale (sprite px -> output px, as `draw_cursor` computes
/// it); `extra` is the animation's scale ON TOP of that. `clip` is the screen-panel box, applied
/// exactly as in `blit`.
pub fn blit_transformed(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite, anchor: (f32, f32),
                        scale: f32, angle_deg: f32, extra: f32, clip: (i32, i32, i32, i32)) {
    let (sw, sh) = (spr.w as f32, spr.h as f32);
    if sw < 1.0 || sh < 1.0 || scale <= 0.0 || extra <= 0.0 { return; }
    let total = scale * extra;
    let (sin, cos) = (angle_deg.to_radians().sin(), angle_deg.to_radians().cos());
    // Hotspot in sprite pixels - the fixed point of both the rotation and the scale.
    let hot = (spr.hot.0 * sw, spr.hot.1 * sh);

    let (x0, y0, x1, y1) = bounds(sw, sh, hot, anchor, total, sin, cos, ow, oh, clip);
    if x0 >= x1 || y0 >= y1 { return; }
    for oy in y0..y1 {
        for ox in x0..x1 {
            // Inverse map: output px -> unrotated, unscaled sprite px.
            let (dx, dy) = (ox as f32 + 0.5 - anchor.0, oy as f32 + 0.5 - anchor.1);
            let (ux, uy) = (dx * cos + dy * sin, -dx * sin + dy * cos);
            let (sx, sy) = (hot.0 + ux / total, hot.1 + uy / total);
            let Some(px) = sample(spr, sx - 0.5, sy - 0.5) else { continue };
            blend(out, ow, ox, oy, px);
        }
    }
}

/// The output rectangle the transformed sprite can touch: its four corners mapped forward, then
/// clamped to the frame and the clip box. Keeps the per-pixel loop off the rest of the frame.
#[allow(clippy::too_many_arguments)]
fn bounds(sw: f32, sh: f32, hot: (f32, f32), anchor: (f32, f32), total: f32, sin: f32, cos: f32,
          ow: u32, oh: u32, clip: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let corners = [(0.0, 0.0), (sw, 0.0), (sw, sh), (0.0, sh)];
    let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
    for (cx, cy) in corners {
        let (vx, vy) = ((cx - hot.0) * total, (cy - hot.1) * total);
        let p = (anchor.0 + vx * cos - vy * sin, anchor.1 + vx * sin + vy * cos);
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    (
        (lo.0.floor() as i32).max(0).max(clip.0),
        (lo.1.floor() as i32).max(0).max(clip.1),
        (hi.0.ceil() as i32).min(ow as i32).min(clip.2),
        (hi.1.ceil() as i32).min(oh as i32).min(clip.3),
    )
}

/// Bilinear sample at sprite-space `(x, y)` (already shifted to texel centres), as PREMULTIPLIED
/// BGRA floats. `None` outside the sprite, or where the result is fully transparent.
///
/// *Why premultiplied:* interpolating straight-alpha colour pulls the RGB of fully transparent
/// texels into the edge, haloing every rotated sprite with whatever its padding happens to be.
fn sample(spr: &CursorSprite, x: f32, y: f32) -> Option<[f32; 4]> {
    let (sw, sh) = (spr.w as i32, spr.h as i32);
    let (fx, fy) = (x.floor(), y.floor());
    if fx < -1.0 || fy < -1.0 || fx >= sw as f32 || fy >= sh as f32 { return None; }
    let (tx, ty) = (x - fx, y - fy);
    let mut acc = [0.0f32; 4];
    for (i, (ox, oy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
        let w = [(1.0 - tx) * (1.0 - ty), tx * (1.0 - ty), (1.0 - tx) * ty, tx * ty][i];
        if w <= 0.0 { continue; }
        let (px, py) = (fx as i32 + ox, fy as i32 + oy);
        if px < 0 || py < 0 || px >= sw || py >= sh { continue; }
        let o = (py as usize * spr.w as usize + px as usize) * 4;
        let a = spr.bgra[o + 3] as f32 / 255.0;
        acc[0] += spr.bgra[o] as f32 * a * w;
        acc[1] += spr.bgra[o + 1] as f32 * a * w;
        acc[2] += spr.bgra[o + 2] as f32 * a * w;
        acc[3] += a * w;
    }
    (acc[3] > 0.002).then_some(acc)
}

/// Source-over blend of one premultiplied BGRA sample onto the frame.
///
/// Rounds rather than truncating (which `blit`'s integer-aligned fast path can afford to do):
/// bilinear weights never land exactly on 1, so an untruncated full-coverage pixel would come out
/// one step dark on every channel - visible as a dimmed rim right around a rotating cursor.
fn blend(out: &mut [u8], ow: u32, ox: i32, oy: i32, px: [f32; 4]) {
    let o = (oy as usize * ow as usize + ox as usize) * 4;
    if o + 3 >= out.len() { return; }
    let ia = 1.0 - px[3].min(1.0);
    for c in 0..3 {
        out[o + c] = (out[o + c] as f32 * ia + px[c]).round().clamp(0.0, 255.0) as u8;
    }
    out[o + 3] = (out[o + 3] as f32 * ia + 255.0 * px[3]).round().clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
#[path = "cursorxform_tests.rs"]
mod tests;
