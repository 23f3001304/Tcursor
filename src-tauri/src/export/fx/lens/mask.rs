use crate::events::track::cursortype::CursorType;
use crate::export::cursor::cursordraw::CursorSprite;
use crate::export::fx::click::clickfx;

#[derive(Clone, Debug, PartialEq)]
pub struct LensMask {
    pub key: u64,
    pub w: u32,
    pub h: u32,
    pub a: Vec<u8>,
}

const MASK_IN: f32 = 0.02;
const MASK_FULL: f32 = 0.10;

pub fn mask_of(pack: &str, kind: CursorType, spr: &CursorSprite) -> LensMask {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    pack.hash(&mut h);
    kind.hash(&mut h);
    let fill =
        |a: u8| -> u8 { (clickfx::smoothstep(MASK_IN, MASK_FULL, a as f32 / 255.0) * 255.0) as u8 };
    LensMask {
        key: h.finish(),
        w: spr.w,
        h: spr.h,
        a: spr.bgra.chunks_exact(4).map(|p| fill(p[3])).collect(),
    }
}

const MORPH_MASK: u32 = 128;

fn tap(m: &LensMask, u: f32, v: f32) -> f32 {
    let x = ((u * m.w as f32) as u32).min(m.w.saturating_sub(1));
    let y = ((v * m.h as f32) as u32).min(m.h.saturating_sub(1));
    m.a.get((y * m.w + x) as usize).map_or(0.0, |&a| a as f32)
}

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
    LensMask {
        key: prev.key ^ cur.key.rotate_left(17) ^ (((t * 255.0) as u64) << 48),
        w: n,
        h: n,
        a,
    }
}
