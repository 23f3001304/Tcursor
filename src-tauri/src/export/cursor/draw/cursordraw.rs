use crate::export::cursor::draw::busy::BusyPose;

pub struct CursorSprite {
    pub bgra: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub hot: (f32, f32),
    pub canvas_h: u32,
}

pub fn decode_sprite(png: &[u8], hot: (f32, f32)) -> Option<CursorSprite> {
    let (bgra, w, h, hot, canvas_h) = crate::export::pipeline::ffio::decode_cursor(png, hot)?;
    Some(CursorSprite {
        bgra,
        w,
        h,
        hot,
        canvas_h,
    })
}

pub fn bounce_scale(click_ms: &[u32], t_ms: u32, enabled: bool, intensity: f32) -> f32 {
    if !enabled || click_ms.is_empty() {
        return 1.0;
    }
    let recent = click_ms.iter().rev().find(|&&c| c <= t_ms);
    match recent {
        None => 1.0,
        Some(&c) => {
            let dt = t_ms - c;
            if dt > 180 {
                1.0
            } else {
                1.0 - 0.36 * intensity.clamp(0.0, 1.0) * (1.0 - dt as f32 / 180.0)
            }
        }
    }
}

fn blit(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    top_left: (f32, f32),
    scale: f32,
    alpha_mul: f32,
    clip: (i32, i32, i32, i32),
) {
    blit_into(
        out,
        ow,
        oh,
        spr,
        [
            top_left.0,
            top_left.1,
            spr.w as f32 * scale,
            spr.h as f32 * scale,
        ],
        alpha_mul,
        clip,
    );
}

pub fn blit_into(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    dest: [f32; 4],
    alpha_mul: f32,
    clip: (i32, i32, i32, i32),
) {
    let tw = dest[2].max(1.0).round() as i32;
    let th = dest[3].max(1.0).round() as i32;
    let tx0 = dest[0].round() as i32;
    let ty0 = dest[1].round() as i32;

    let ox_start = tx0.max(0).max(clip.0);
    let oy_start = ty0.max(0).max(clip.1);
    let ox_end = (tx0 + tw).min(ow as i32).min(clip.2);
    let oy_end = (ty0 + th).min(oh as i32).min(clip.3);

    if ox_start >= ox_end || oy_start >= oy_end {
        return;
    }

    let sw = spr.w as i32;
    let sh = spr.h as i32;

    for oy in oy_start..oy_end {
        let sy = ((oy - ty0) * sh / th).clamp(0, sh - 1) as usize;
        for ox in ox_start..ox_end {
            let sx = ((ox - tx0) * sw / tw).clamp(0, sw - 1) as usize;
            let src_off = (sy * spr.w as usize + sx) * 4;
            let dst_off = (oy as usize * ow as usize + ox as usize) * 4;
            if src_off + 3 >= spr.bgra.len() || dst_off + 3 >= out.len() {
                continue;
            }
            let sa = spr.bgra[src_off + 3] as f32 / 255.0 * alpha_mul;
            if sa <= 0.0 {
                continue;
            }
            let ia = 1.0 - sa;
            out[dst_off] = (out[dst_off] as f32 * ia + spr.bgra[src_off] as f32 * sa) as u8;
            out[dst_off + 1] =
                (out[dst_off + 1] as f32 * ia + spr.bgra[src_off + 1] as f32 * sa) as u8;
            out[dst_off + 2] =
                (out[dst_off + 2] as f32 * ia + spr.bgra[src_off + 2] as f32 * sa) as u8;
            out[dst_off + 3] = (out[dst_off + 3] as f32 * ia + 255.0 * sa) as u8;
        }
    }
}

pub fn draw_cursor(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    pos: (f32, f32),
    recent: &[(f32, f32)],
    size_px: f32,
    blur: f32,
    bounce: f32,
    clip: (i32, i32, i32, i32),
) {
    draw_cursor_posed(
        out,
        ow,
        oh,
        spr,
        pos,
        recent,
        size_px,
        blur,
        bounce,
        clip,
        BusyPose::still(),
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_cursor_posed(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    pos: (f32, f32),
    recent: &[(f32, f32)],
    size_px: f32,
    blur: f32,
    bounce: f32,
    clip: (i32, i32, i32, i32),
    pose: BusyPose,
    alpha: f32,
) {
    let scale = (size_px * bounce / spr.canvas_h as f32).max(0.0001);
    let th = spr.h as f32 * scale;
    let tw = spr.w as f32 * scale;
    let hx = spr.hot.0 * tw;
    let hy = spr.hot.1 * th;
    let top_left = (pos.0 - hx, pos.1 - hy);

    let n = recent.len();
    if n > 0 && blur > 0.0 {
        let mut last_draw = pos;
        for (i, rp) in recent.iter().enumerate() {
            let dist = (rp.0 - last_draw.0).hypot(rp.1 - last_draw.1);
            if dist < 1.5 {
                continue;
            }
            let fade = 1.0 - i as f32 / n as f32;
            let a = (blur * fade * 0.5).clamp(0.0, 1.0) * alpha;
            let rtl = (rp.0 - hx, rp.1 - hy);
            blit(out, ow, oh, spr, rtl, scale, a, clip);
            last_draw = *rp;
        }
    }

    if pose.is_identity() {
        blit(out, ow, oh, spr, top_left, scale, alpha, clip);
    } else {
        crate::export::cursor::draw::cursorxform::blit_transformed(
            out,
            ow,
            oh,
            spr,
            pos,
            scale,
            pose.angle_deg,
            pose.scale,
            clip,
            alpha,
        );
    }
}

pub fn apply_enhanced(
    out: &mut [u8],
    ow: u32,
    oh: u32,
    spr: &CursorSprite,
    pos_px: (f32, f32),
    recent: &mut std::collections::VecDeque<(f32, f32)>,
    trail_cap: usize,
    click_ms: &[u32],
    ev_t: u32,
    size: f32,
    blur: f32,
    click_bounce: bool,
    bounce_intensity: f32,
    panel: f32,
    clip: (i32, i32, i32, i32),
    pose: BusyPose,
    alpha: f32,
) {
    if recent.len() >= trail_cap {
        recent.pop_front();
    }
    recent.push_back(pos_px);
    let bounce = bounce_scale(click_ms, ev_t, click_bounce, bounce_intensity);
    let size_px = size.clamp(0.4, 3.0) * oh as f32 * 0.033 * panel;
    let trail: Vec<(f32, f32)> = recent.iter().rev().skip(1).copied().collect();
    draw_cursor_posed(
        out,
        ow,
        oh,
        spr,
        pos_px,
        &trail,
        size_px,
        blur.clamp(0.0, 1.0),
        bounce,
        clip,
        pose,
        alpha.clamp(0.0, 1.0),
    );
}

#[cfg(test)]
#[path = "cursordraw_tests.rs"]
mod tests;
