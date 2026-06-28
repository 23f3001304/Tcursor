// Synthetic cursor drawing: sprite decode, click-bounce curve, alpha blit with trail.
// CPU-only - the cursor occupies a small region of the frame.

use crate::events::model::EventKind;
use crate::settings::model::CursorStyle;

/// Pre-computed state for Enhanced cursor drawing, allocated once before the export loop.
pub struct CursorPrep {
    pub sprite: CursorSprite,
    pub click_ms: Vec<u32>,
    pub recent: std::collections::VecDeque<(f32, f32)>,
}

/// Build a CursorPrep if the cursor style is Enhanced; returns None for System/Hidden.
pub fn prep(
    cursor: &crate::settings::model::CursorSettings,
    events: &[crate::events::model::MouseEvent],
    png: &[u8],
) -> Option<CursorPrep> {
    if cursor.style != CursorStyle::Enhanced { return None; }
    let sprite = decode_sprite(png)?;
    let click_ms: Vec<u32> = events.iter()
        .filter(|e| e.kind == EventKind::Down).map(|e| e.t).collect();
    let recent = std::collections::VecDeque::new();
    Some(CursorPrep { sprite, click_ms, recent })
}

pub struct CursorSprite {
    pub bgra: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

/// Decode the bundled pointer PNG to a native-size BGRA CursorSprite.
/// The PNG is 114x174 (white arrow, hotspot at top-left tip).
/// Returns None if decode fails.
pub fn decode_sprite(png: &[u8]) -> Option<CursorSprite> {
    match crate::export::ffio::decode_image(png, 114, 174) {
        Ok(bgra) => Some(CursorSprite { bgra, w: 114, h: 174 }),
        Err(_) => None,
    }
}

/// Scale factor for the cursor at time `t_ms`, given the list of click timestamps.
/// - Returns 1.0 when `enabled` is false, or no click is within 180 ms before `t_ms`.
/// - Otherwise: linearly dips from ~0.82 at t=click to 1.0 at t=click+180ms.
pub fn bounce_scale(click_ms: &[u32], t_ms: u32, enabled: bool) -> f32 {
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
                1.0 - 0.18 * (1.0 - dt as f32 / 180.0)
            }
        }
    }
}

/// Alpha-blit a scaled copy of `spr` onto `out` (BGRA, `ow x oh`).
/// `top_left`: output pixel coordinate of the sprite's top-left corner.
/// `scale`: sprite height in output pixels / sprite natural height.
/// `alpha_mul`: multiplier on the sprite's alpha channel (0.0 = invisible, 1.0 = full).
fn blit(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
        top_left: (f32, f32), scale: f32, alpha_mul: f32) {
    let tw = (spr.w as f32 * scale).max(1.0).round() as i32;
    let th = (spr.h as f32 * scale).max(1.0).round() as i32;
    let tx0 = top_left.0.round() as i32;
    let ty0 = top_left.1.round() as i32;

    let ox_start = tx0.max(0);
    let oy_start = ty0.max(0);
    let ox_end = (tx0 + tw).min(ow as i32);
    let oy_end = (ty0 + th).min(oh as i32);

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
/// `pos`: on-screen position of the cursor hotspot (top-left tip ~6%x, 4%y of sprite).
/// `recent`: previous hotspot positions for the motion trail (newest first after caller reverses).
/// `size_px`: target height of the sprite in output pixels.
/// `blur`: trail strength 0..1 (0 = no trail).
/// `bounce`: scale multiplier from bounce_scale (dip on click).
pub fn draw_cursor(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
                   pos: (f32, f32), recent: &[(f32, f32)],
                   size_px: f32, blur: f32, bounce: f32) {
    let th = (size_px * bounce).max(1.0);
    let scale = th / spr.h as f32;
    let tw = spr.w as f32 * scale;
    let hx = 0.06 * tw;
    let hy = 0.04 * th;
    let top_left = (pos.0 - hx, pos.1 - hy);

    // Draw trail (older positions = more faded)
    let n = recent.len();
    if n > 0 && blur > 0.0 {
        for (i, rp) in recent.iter().enumerate() {
            let fade = 1.0 - i as f32 / n as f32;
            let alpha = (blur * fade * 0.5).clamp(0.0, 1.0);
            let rtl = (rp.0 - hx, rp.1 - hy);
            blit(out, ow, oh, spr, rtl, scale, alpha);
        }
    }

    // Draw the main cursor on top
    blit(out, ow, oh, spr, top_left, scale, 1.0);
}

/// Per-frame helper: updates `recent` trail, computes bounce + size, and calls draw_cursor.
/// `pos_px`: projected on-screen cursor position (output pixels).
/// `recent`: mutable trail deque (oldest first); capped at `trail_cap` entries.
/// `click_ms`: timestamps of Down events for bounce.
/// `ev_t`: current export frame time in ms (event-relative).
/// `oh`: output frame height (used to derive default cursor height from `size`).
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
) {
    if recent.len() >= trail_cap { recent.pop_front(); }
    recent.push_back(pos_px);
    let bounce = bounce_scale(click_ms, ev_t, click_bounce);
    let size_px = size.clamp(0.4, 3.0) * oh as f32 * 0.022;
    let trail: Vec<(f32, f32)> = recent.iter().rev().skip(1).copied().collect();
    draw_cursor(out, ow, oh, spr, pos_px, &trail, size_px, blur.clamp(0.0, 1.0), bounce);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spr() -> CursorSprite { CursorSprite { bgra: vec![255u8; 4 * 4 * 4], w: 4, h: 4 } }

    #[test]
    fn bounce_is_identity_when_disabled_or_idle() {
        assert_eq!(bounce_scale(&[], 1000, true), 1.0);
        assert_eq!(bounce_scale(&[500], 1000, false), 1.0); // disabled
        assert_eq!(bounce_scale(&[100], 5000, true), 1.0);  // click long past -> recovered
    }
    #[test]
    fn bounce_dips_right_after_a_click() {
        let s = bounce_scale(&[1000], 1010, true); // 10ms after a click
        assert!(s < 1.0 && s > 0.5, "dips below 1.0 just after a click, got {s}");
    }
    #[test]
    fn draws_pixels_at_the_position() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw_cursor(&mut out, w, h, &spr(), (20.0, 20.0), &[], 8.0, 0.0, 1.0);
        assert!(out.iter().any(|&b| b > 0), "cursor blit wrote visible pixels");
    }
    #[test]
    fn offscreen_position_is_safe_noop() {
        let (w, h) = (40u32, 40u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw_cursor(&mut out, w, h, &spr(), (1000.0, 1000.0), &[], 8.0, 0.0, 1.0); // must not panic / OOB
        assert!(out.iter().all(|&b| b == 0));
    }
}
