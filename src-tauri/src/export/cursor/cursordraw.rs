// Synthetic cursor drawing: sprite decode, click-bounce curve, alpha blit with trail.
// CPU-only - the cursor occupies a small region of the frame.
// The per-type sprite set + theme invert live in `cursorset`; this file is the draw seam.

pub struct CursorSprite {
    pub bgra: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub hot: (f32, f32),
    pub canvas_h: u32,
}

/// Decode a cursor PNG to a content-tight sprite; `hot` (canvas-fraction) is re-based to content.
pub fn decode_sprite(png: &[u8], hot: (f32, f32)) -> Option<CursorSprite> {
    let (bgra, w, h, hot, canvas_h) = crate::export::pipeline::ffio::decode_cursor(png, hot)?;
    Some(CursorSprite { bgra, w, h, hot, canvas_h })
}

/// Scale factor for the cursor at time `t_ms`, given the list of click timestamps.
/// - Returns 1.0 when `enabled` is false, or no click is within 180 ms before `t_ms`.
/// - Otherwise: linearly dips after a click; dip depth = 0.36 * intensity.clamp(0,1).
///   At default intensity 0.5 this reproduces the original ~0.18 dip.
pub fn bounce_scale(click_ms: &[u32], t_ms: u32, enabled: bool, intensity: f32) -> f32 {
    if !enabled || click_ms.is_empty() {
        return 1.0;
    }
    // Find the most recent click at or before t_ms
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

/// Alpha-blit a scaled copy of `spr` onto `out` (BGRA, `ow x oh`).
/// `top_left`: output pixel coordinate of the sprite's top-left corner.
/// `scale`: sprite height in output pixels / sprite natural height.
/// `alpha_mul`: multiplier on the sprite's alpha channel (0.0 = invisible, 1.0 = full).
/// `clip`: output bounds (x0,y0,x1,y1) the blit is confined to (the screen panel rect).
fn blit(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
        top_left: (f32, f32), scale: f32, alpha_mul: f32, clip: (i32, i32, i32, i32)) {
    let tw = (spr.w as f32 * scale).max(1.0).round() as i32;
    let th = (spr.h as f32 * scale).max(1.0).round() as i32;
    let tx0 = top_left.0.round() as i32;
    let ty0 = top_left.1.round() as i32;

    // Confine to the clip rect (screen panel) as well as the frame.
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
            if sa <= 0.0 { continue; }
            let ia = 1.0 - sa;
            out[dst_off]     = (out[dst_off]     as f32 * ia + spr.bgra[src_off]     as f32 * sa) as u8;
            out[dst_off + 1] = (out[dst_off + 1] as f32 * ia + spr.bgra[src_off + 1] as f32 * sa) as u8;
            out[dst_off + 2] = (out[dst_off + 2] as f32 * ia + spr.bgra[src_off + 2] as f32 * sa) as u8;
            out[dst_off + 3] = (out[dst_off + 3] as f32 * ia + 255.0 * sa) as u8;
        }
    }
}

/// Draw the cursor sprite (and its motion trail) onto `out` (BGRA, `ow x oh`).
/// `pos`: on-screen position of the cursor hotspot (sprite-supplied; see CursorSprite.hot).
/// `recent`: previous hotspot positions for the motion trail (newest first after caller reverses).
/// `size_px`: target full-canvas height in output px; content scales within it (canvas-relative).
/// `blur`: trail strength 0..1 (0 = no trail).
/// `bounce`: scale multiplier from bounce_scale (dip on click).
/// `clip`: output bounds (x0,y0,x1,y1) the cursor + trail are confined to (the screen panel).
pub fn draw_cursor(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
                   pos: (f32, f32), recent: &[(f32, f32)],
                   size_px: f32, blur: f32, bounce: f32, clip: (i32, i32, i32, i32)) {
    // Scale by the full canvas so every pack cursor keeps its authored relative size
    // (a wide resize arrow stays wide, not stretched to the pointer's height).
    let scale = (size_px * bounce / spr.canvas_h as f32).max(0.0001);
    let th = spr.h as f32 * scale;
    let tw = spr.w as f32 * scale;
    let hx = spr.hot.0 * tw;
    let hy = spr.hot.1 * th;
    let top_left = (pos.0 - hx, pos.1 - hy);

    // Draw trail (older positions = more faded) - only for points moving at least 1.5px
    let n = recent.len();
    if n > 0 && blur > 0.0 {
        let mut last_draw = pos;
        for (i, rp) in recent.iter().enumerate() {
            let dist = (rp.0 - last_draw.0).hypot(rp.1 - last_draw.1);
            if dist < 1.5 { continue; }
            let fade = 1.0 - i as f32 / n as f32;
            let alpha = (blur * fade * 0.5).clamp(0.0, 1.0);
            let rtl = (rp.0 - hx, rp.1 - hy);
            blit(out, ow, oh, spr, rtl, scale, alpha, clip);
            last_draw = *rp;
        }
    }

    // Draw the main cursor on top
    blit(out, ow, oh, spr, top_left, scale, 1.0, clip);
}

/// Per-frame helper: updates trail, computes bounce + size, calls draw_cursor.
/// `oh` used to derive cursor height from `size`; `bounce_intensity` scales dip depth.
pub fn apply_enhanced(
    out: &mut [u8], ow: u32, oh: u32,
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
) {
    if recent.len() >= trail_cap { recent.pop_front(); }
    recent.push_back(pos_px);
    let bounce = bounce_scale(click_ms, ev_t, click_bounce, bounce_intensity);
    let size_px = size.clamp(0.4, 3.0) * oh as f32 * 0.033 * panel;
    let trail: Vec<(f32, f32)> = recent.iter().rev().skip(1).copied().collect();
    draw_cursor(out, ow, oh, spr, pos_px, &trail, size_px, blur.clamp(0.0, 1.0), bounce, clip);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spr() -> CursorSprite { CursorSprite { bgra: vec![255u8; 4 * 4 * 4], w: 4, h: 4, hot: (0.0, 0.0), canvas_h: 4 } }

    #[test]
    fn bounce_is_identity_when_disabled_or_idle() {
        assert_eq!(bounce_scale(&[], 1000, true, 0.5), 1.0);
        assert_eq!(bounce_scale(&[500], 1000, false, 0.5), 1.0); // disabled
        assert_eq!(bounce_scale(&[100], 5000, true, 0.5), 1.0);  // click long past -> recovered
    }
    #[test]
    fn bounce_dips_right_after_a_click() {
        let s = bounce_scale(&[1000], 1010, true, 0.5); // 10ms after a click
        assert!(s < 1.0 && s > 0.5, "dips below 1.0 just after a click, got {s}");
    }
    #[test]
    fn higher_intensity_produces_deeper_dip() {
        let deep = bounce_scale(&[1000], 1000, true, 1.0);
        let shallow = bounce_scale(&[1000], 1000, true, 0.3);
        assert!(deep < shallow, "intensity 1.0 should dip deeper than 0.3, got {deep} vs {shallow}");
    }
    #[test]
    fn draws_pixels_at_the_position() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw_cursor(&mut out, w, h, &spr(), (20.0, 20.0), &[], 8.0, 0.0, 1.0, (0, 0, w as i32, h as i32));
        assert!(out.iter().any(|&b| b > 0), "cursor blit wrote visible pixels");
    }
    /// The ghost-cursor mechanism `FrameRenderer::reset_camera` now clears: a stale `recent` entry
    /// (a PREVIOUS scrub target) blits a faded sprite THERE. Clearing it removes the ghost.
    #[test]
    fn a_stale_trail_point_draws_a_ghost_until_recent_is_cleared() {
        let mut recent = std::collections::VecDeque::from(vec![(4.0f32, 4.0f32)]);
        let ghost_alpha = |r: &mut std::collections::VecDeque<(f32, f32)>| {
            let mut out = vec![0u8; 40 * 40 * 4];
            apply_enhanced(&mut out, 40, 40, &spr(), (30.0, 30.0), r, 6, &[], 0, 1.0, 0.9, false, 0.5, 1.0, (0, 0, 40, 40));
            out[(4 * 40 + 4) * 4 + 3]
        };
        assert!(ghost_alpha(&mut recent) > 0, "a stale trail point paints a ghost cursor at (4,4)");
        recent.clear();
        assert_eq!(ghost_alpha(&mut recent), 0, "a cleared trail leaves no ghost");
    }
    #[test]
    fn offscreen_position_is_safe_noop() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw_cursor(&mut out, w, h, &spr(), (1000.0, 1000.0), &[], 8.0, 0.0, 1.0, (0, 0, w as i32, h as i32)); // must not panic / OOB
        assert!(out.iter().all(|&b| b == 0));
    }
}
