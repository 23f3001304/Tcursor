// Cross-fading one cursor STATE into the next, for a `material: "glass"` pack only.
//
// Crystal's shapes are objects, not pictures of pointers - a frosted disc, a pill along a line of
// text, an oriented capsule - so snapping from one to the next reads as a different object being
// swapped in. Instead the placed box is interpolated from the previous kind's to the new one's over
// `fx_lens::MORPH_MS`, both sprites are drawn stretched into THAT box, and their alphas cross-fade
// through it. The lens refraction rides the same box and the same blend (`fx_lens::morph_mask`), so
// the bent frame morphs with the glass rather than jumping under it.
//
// Plain packs never come here: their cursors are pictures of pointers, where a snap is what an OS
// cursor does and a dissolve would look like a bug.
use crate::events::track::cursortype::CursorType;
use crate::export::cursor::cursordraw::{blit_into, bounce_scale, CursorSprite};
use crate::export::cursor::cursorset::CursorPrep;
use crate::export::cursor::cursorxform::blit_transformed;
use crate::settings::cursor::CursorSettings;

/// A sprite's placed box in OUTPUT px as `[x0, y0, w, h]` - the hotspot lands on `pos`, and the
/// scale is the canvas-relative one every cursor uses (`cursordraw::draw_cursor_posed`). THE one
/// definition: `fx_lensbuild` places the lens with it and `draw_glass` below blits with it, so the
/// glass and the frame it bends can never disagree about where the cursor is.
pub fn sprite_box(spr: &CursorSprite, pos: (f32, f32), size_px: f32, bounce: f32) -> [f32; 4] {
    let scale = (size_px * bounce / spr.canvas_h.max(1) as f32).max(0.0001);
    let (w, h) = (spr.w as f32 * scale, spr.h as f32 * scale);
    [pos.0 - spr.hot.0 * w, pos.1 - spr.hot.1 * h, w, h]
}

/// `a` lerped to `b` by `m` (already eased by `fx_lens::kind_morph`), component-wise.
pub fn lerp_box(a: [f32; 4], b: [f32; 4], m: f32) -> [f32; 4] {
    let t = m.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t,
     a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
}

/// The box the cursor occupies this frame: `cur`'s own, or the interpolation from `prev`'s while a
/// kind change is still easing. `size_px`/`bounce` are the values `apply_enhanced` would compute.
pub fn morph_box(prev: &CursorSprite, cur: &CursorSprite, m: f32, pos: (f32, f32),
                 size_px: f32, bounce: f32) -> [f32; 4] {
    let b = sprite_box(cur, pos, size_px, bounce);
    if m >= 1.0 { return b; }
    lerp_box(sprite_box(prev, pos, size_px, bounce), b, m)
}

/// One sprite, stretched into `dest` at `alpha`. A non-zero `angle_deg` (a spinning busy ring, or a
/// morph on its way into or out of one) needs the rotated blit, which only takes a UNIFORM scale -
/// there it is height-matched to `dest` and the aspect stretch is dropped. That only ever applies
/// while a rotation is in flight, where nothing is settled enough to read an aspect off.
fn one(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite, dest: [f32; 4], alpha: f32,
       angle_deg: f32, clip: (i32, i32, i32, i32)) {
    if alpha <= 0.002 { return; }
    if angle_deg == 0.0 { blit_into(out, ow, oh, spr, dest, alpha, clip); return; }
    let anchor = (dest[0] + spr.hot.0 * dest[2], dest[1] + spr.hot.1 * dest[3]);
    let scale = (dest[3] / spr.h.max(1) as f32).max(0.0001);
    blit_transformed(out, ow, oh, spr, anchor, scale, angle_deg, 1.0, clip, alpha);
}

/// The whole glass-pack cursor draw for one frame: motion trail, then the two states cross-faded
/// inside one interpolated box. `m` is `fx_lens::kind_morph`'s eased progress and `angle_deg` the
/// busy rotation already scaled by it - both from `fx_lensbuild::morph_at`, which is also what
/// placed the lens, so the two cannot disagree.
///
/// A glass pack shipping EXPLICIT `busy_NN.png` frames is the one thing this path does not honour:
/// it draws the pack's `busy.png` under the synthesised rotation instead, because the lens mask is
/// per-kind and an explicit frame has no mask of its own. No shipped glass pack has frames.
#[allow(clippy::too_many_arguments)]
pub fn draw_glass(cp: &mut CursorPrep, out: &mut [u8], ow: u32, oh: u32, pos: (f32, f32),
                  prev_kind: CursorType, cur_kind: CursorType, m: f32, angle_deg: f32,
                  panel: f32, clip: (i32, i32, i32, i32), ev_t: u32, c: &CursorSettings,
                  alpha: f32) {
    // Disjoint field borrows: the sprites come out of `cp.set` while `cp.recent` is still borrowed
    // mutably for the trail - the same trick `cursorset::sprite_for` exists for.
    let fallback = cp.set.get(&CursorType::Arrow);
    let (Some(cur), Some(prev)) = (cp.set.get(&cur_kind).or(fallback), cp.set.get(&prev_kind).or(fallback))
        else { return };
    if cp.recent.len() >= 6 { cp.recent.pop_front(); }
    cp.recent.push_back(pos);
    let bounce = bounce_scale(&cp.click_ms, ev_t, c.click_bounce, c.bounce_intensity);
    let size_px = c.size.clamp(0.4, 3.0) * oh as f32 * 0.033 * panel;
    let dest = morph_box(prev, cur, m, pos, size_px, bounce);
    // The trail is the CURRENT state only, and never cross-faded: it is a fading echo of where the
    // cursor WAS, and dissolving each ghost through a second shape reads as noise.
    let blur = c.motion_blur.clamp(0.0, 1.0);
    if blur > 0.0 {
        let (mut last, n) = (pos, cp.recent.len());
        let trail: Vec<(f32, f32)> = cp.recent.iter().rev().skip(1).copied().collect();
        for (i, rp) in trail.iter().enumerate() {
            if (rp.0 - last.0).hypot(rp.1 - last.1) < 1.5 { continue; }
            let a = (blur * (1.0 - i as f32 / n as f32) * 0.5).clamp(0.0, 1.0) * alpha;
            let at = [dest[0] + rp.0 - pos.0, dest[1] + rp.1 - pos.1, dest[2], dest[3]];
            one(out, ow, oh, cur, at, a, 0.0, clip);
            last = *rp;
        }
    }
    one(out, ow, oh, prev, dest, alpha * (1.0 - m), angle_deg, clip);
    one(out, ow, oh, cur, dest, alpha * m, angle_deg, clip);
}

#[cfg(test)]
#[path = "cursormorph_tests.rs"]
mod tests;
