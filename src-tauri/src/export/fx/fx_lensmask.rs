// The lens MASK: which pixels of a frame the glass covers, and how one cursor state's silhouette
// dissolves into the next. Split from `fx_lens.rs` (the shapes and the curves) for the size limit;
// both halves are `pub use`d from there, so callers keep saying `fx_lens::mask_of`.
use crate::events::track::cursortype::CursorType;
use crate::export::cursor::cursordraw::CursorSprite;

/// One cursor sprite's alpha channel, ready to be a shader mask. `key` identifies the (pack, kind)
/// pair so `GpuFx` can upload it once and reuse it for every later frame.
#[derive(Clone, Debug, PartialEq)]
pub struct LensMask { pub key: u64, pub w: u32, pub h: u32, pub a: Vec<u8> }

/// Where a sprite's alpha stops being background and starts being the lens. A clear-glass sprite is
/// a bright opaque RIM around a body of about 10-40% alpha (that faint body IS the glass), so using
/// the alpha verbatim would refract the body at a tenth strength and leave the lens nearly invisible
/// - which is exactly what the first bench frames showed. Saturating between these two makes the
/// whole silhouette the lens while keeping the one-pixel antialiased edge soft.
const MASK_IN: f32 = 0.02;
const MASK_FULL: f32 = 0.10;

/// One decoded sprite's silhouette as a mask, keyed by its pack and kind so the GPU uploads it
/// once. Saturated per `MASK_IN`/`MASK_FULL`, so the mask is "is this the cursor", not "how opaque
/// is the cursor" - the sprite's own opacity still shows, in the 65%-alpha blit that follows.
pub fn mask_of(pack: &str, kind: CursorType, spr: &CursorSprite) -> LensMask {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pack.hash(&mut h);
    kind.hash(&mut h);
    let fill = |a: u8| -> u8 {
        (crate::export::fx::clickfx::smoothstep(MASK_IN, MASK_FULL, a as f32 / 255.0) * 255.0) as u8
    };
    LensMask { key: h.finish(), w: spr.w, h: spr.h,
        a: spr.bgra.chunks_exact(4).map(|p| fill(p[3])).collect() }
}

/// Side of the square buffer a cross-faded mask is resampled into - the packs' own sprite size, so
/// a settled mask is never blurrier than the sprite it came from.
const MORPH_MASK: u32 = 128;

/// Nearest-neighbour sample of `m` at normalized `(u, v)`.
fn tap(m: &LensMask, u: f32, v: f32) -> f32 {
    let x = ((u * m.w as f32) as u32).min(m.w.saturating_sub(1));
    let y = ((v * m.h as f32) as u32).min(m.h.saturating_sub(1));
    m.a.get((y * m.w + x) as usize).map_or(0.0, |&a| a as f32)
}

/// The mask MID-MORPH: both kinds' silhouettes resampled into one square and blended by `p`.
///
/// The two sprites are drawn stretched into the SAME interpolated box (`cursormorph`), so their
/// masks share that box's normalized space and the blend is a plain lerp - no per-kind hotspot or
/// aspect fix-up. One glass state therefore dissolves into the next INCLUDING the refraction, which
/// is the whole point: a snap of the bent frame is more obvious than a snap of the sprite.
pub fn morph_mask(prev: &LensMask, cur: &LensMask, p: f32) -> LensMask {
    let (n, t) = (MORPH_MASK, p.clamp(0.0, 1.0));
    let mut a = Vec::with_capacity((n * n) as usize);
    for y in 0..n {
        let v = (y as f32 + 0.5) / n as f32;
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32;
            a.push((tap(prev, u, v) + (tap(cur, u, v) - tap(prev, u, v)) * t) as u8);
        }
    }
    // The key carries both sources AND the step, so the GPU's one-entry cache re-uploads through a
    // morph (10 frames of 16 KB) and stops the moment it settles.
    LensMask { key: prev.key ^ cur.key.rotate_left(17) ^ (((t * 255.0) as u64) << 48), w: n, h: n, a }
}
