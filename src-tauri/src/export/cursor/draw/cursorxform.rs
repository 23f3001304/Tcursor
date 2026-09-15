use crate::export::cursor::draw::cursordraw::CursorSprite;

#[allow(clippy::too_many_arguments)]
pub fn blit_transformed(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    anchor: (f32, f32),
    scale: f32,
    angle_deg: f32,
    extra: f32,
    clip: (i32, i32, i32, i32),
    alpha: f32,
) {
    let (sw, sh) = (spr.w as f32, spr.h as f32);
    if sw < 1.0 || sh < 1.0 || scale <= 0.0 || extra <= 0.0 {
        return;
    }
    let total = scale * extra;
    let (sin, cos) = (angle_deg.to_radians().sin(), angle_deg.to_radians().cos());
    let hot = (spr.hot.0 * sw, spr.hot.1 * sh);

    let (x0, y0, x1, y1) = bounds(sw, sh, hot, anchor, total, sin, cos, ow, oh, clip);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for oy in y0..y1 {
        for ox in x0..x1 {
            let (dx, dy) = (ox as f32 + 0.5 - anchor.0, oy as f32 + 0.5 - anchor.1);
            let (ux, uy) = (dx * cos + dy * sin, -dx * sin + dy * cos);
            let (sx, sy) = (hot.0 + ux / total, hot.1 + uy / total);
            let Some(px) = sample(spr, sx - 0.5, sy - 0.5) else {
                continue;
            };
            let a = alpha.clamp(0.0, 1.0);
            blend(
                out,
                ow,
                ox,
                oy,
                [px[0] * a, px[1] * a, px[2] * a, px[3] * a],
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bounds(
    sw: f32,
    sh: f32,
    hot: (f32, f32),
    anchor: (f32, f32),
    total: f32,
    sin: f32,
    cos: f32,
    ow: u32,
    oh: u32,
    clip: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let corners = [(0.0, 0.0), (sw, 0.0), (sw, sh), (0.0, sh)];
    let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
    for (cx, cy) in corners {
        let (vx, vy) = ((cx - hot.0) * total, (cy - hot.1) * total);
        let p = (
            anchor.0 + vx * cos - vy * sin,
            anchor.1 + vx * sin + vy * cos,
        );
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

fn sample(spr: &CursorSprite, x: f32, y: f32) -> Option<[f32; 4]> {
    let (sw, sh) = (spr.w as i32, spr.h as i32);
    let (fx, fy) = (x.floor(), y.floor());
    if fx < -1.0 || fy < -1.0 || fx >= sw as f32 || fy >= sh as f32 {
        return None;
    }
    let (tx, ty) = (x - fx, y - fy);
    let mut acc = [0.0f32; 4];
    for (i, (ox, oy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
        let w = [
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ][i];
        if w <= 0.0 {
            continue;
        }
        let (px, py) = (fx as i32 + ox, fy as i32 + oy);
        if px < 0 || py < 0 || px >= sw || py >= sh {
            continue;
        }
        let o = (py as usize * spr.w as usize + px as usize) * 4;
        let a = spr.bgra[o + 3] as f32 / 255.0;
        acc[0] += spr.bgra[o] as f32 * a * w;
        acc[1] += spr.bgra[o + 1] as f32 * a * w;
        acc[2] += spr.bgra[o + 2] as f32 * a * w;
        acc[3] += a * w;
    }
    (acc[3] > 0.002).then_some(acc)
}

fn blend(out: &mut [u8], ow: u32, ox: i32, oy: i32, px: [f32; 4]) {
    let o = (oy as usize * ow as usize + ox as usize) * 4;
    if o + 3 >= out.len() {
        return;
    }
    let ia = 1.0 - px[3].min(1.0);
    for c in 0..3 {
        out[o + c] = (out[o + c] as f32 * ia + px[c]).round().clamp(0.0, 255.0) as u8;
    }
    out[o + 3] = (out[o + 3] as f32 * ia + 255.0 * px[3])
        .round()
        .clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
#[path = "cursorxform_tests.rs"]
mod tests;
