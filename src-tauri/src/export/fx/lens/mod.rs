use crate::events::model::{Button, EventKind, MouseEvent};
use crate::events::track::cursortype::{CursorTrack, CursorType};
use crate::export::fx::click::clickfx;
use std::sync::Arc;

pub mod build;
pub mod draw;
pub mod mask;

pub const GLASS: &str = "glass";

pub const SPRITE_ALPHA: f32 = 0.65;

pub const ZOOM: f32 = 1.35;

pub const BACK_SCALE: f32 = 2.2;

pub const PILL_W: f32 = 0.35;

pub const SQUASH: f32 = 0.92;
pub const SQUASH_MS: f32 = 120.0;

pub const INK_MS: f32 = 260.0;

pub const MORPH_MS: f32 = 160.0;

pub use self::mask::{mask_of, morph_mask, LensMask};

#[derive(Clone, Debug, PartialEq)]
pub struct CursorLens {
    pub cbox: [f32; 4],
    pub angle: f32,
    pub squash: f32,
    pub ink: f32,
    pub ink_at: [f32; 2],
    pub mask: Arc<LensMask>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackLens {
    pub mn: [f32; 2],
    pub mx: [f32; 2],
    pub r: f32,
    pub squash: f32,
    pub ink: f32,
    pub ink_at: [f32; 2],
    pub ring: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lenses {
    pub glass: Option<CursorLens>,
    pub back: Option<BackLens>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSpan {
    pub down: u32,
    pub up: u32,
    pub x: i32,
    pub y: i32,
}

fn ease(p: f32) -> f32 {
    clickfx::ease_out(p)
}

fn since_click(clicks: &[u32], t: u32) -> Option<f32> {
    clicks
        .iter()
        .rev()
        .find(|&&c| c <= t)
        .map(|&c| (t - c) as f32)
}

pub fn squash_at(clicks: &[u32], t: u32) -> f32 {
    match since_click(clicks, t) {
        Some(dt) if dt < SQUASH_MS => SQUASH + (1.0 - SQUASH) * (dt / SQUASH_MS),
        _ => 1.0,
    }
}

pub fn ink_at(clicks: &[u32], t: u32) -> f32 {
    match since_click(clicks, t) {
        Some(dt) if dt < INK_MS => dt / INK_MS,
        _ => -1.0,
    }
}

pub fn drag_spans(events: &[MouseEvent]) -> Vec<DragSpan> {
    let mut out: Vec<DragSpan> = Vec::new();
    for e in events {
        if e.button != Some(Button::Left) {
            continue;
        }
        match e.kind {
            EventKind::Down => out.push(DragSpan {
                down: e.t,
                up: u32::MAX,
                x: e.x,
                y: e.y,
            }),
            EventKind::Up => {
                if let Some(last) = out.last_mut() {
                    if last.up == u32::MAX {
                        last.up = e.t;
                    }
                }
            }
            EventKind::Move => {}
        }
    }
    out
}

pub fn selection_at(spans: &[DragSpan], t: u32) -> Option<([i32; 2], f32)> {
    let i = spans.partition_point(|s| s.down <= t);
    let s = spans.get(i.checked_sub(1)?)?;
    let p = |ms: f32| ease((ms / MORPH_MS).clamp(0.0, 1.0));
    if s.up > t {
        return Some(([s.x, s.y], p((t - s.down) as f32)));
    }
    let after = (t - s.up) as f32;
    (after < MORPH_MS).then(|| ([s.x, s.y], 1.0 - p(after)))
}

pub fn kind_morph(track: &CursorTrack, t: u32) -> (CursorType, CursorType, f32) {
    let i = track.samples.partition_point(|&(s, _)| s <= t);
    if i == 0 {
        return (CursorType::Arrow, CursorType::Arrow, 1.0);
    }
    let (at, kind) = track.samples[i - 1];
    let prev = if i >= 2 {
        track.samples[i - 2].1
    } else {
        CursorType::Arrow
    };
    (
        kind,
        prev,
        ease(((t.saturating_sub(at)) as f32 / MORPH_MS).clamp(0.0, 1.0)),
    )
}

fn back_of(kind: CursorType, h: f32) -> [f32; 2] {
    let rad = h * BACK_SCALE * 0.5;
    match kind {
        CursorType::IBeam => [rad, rad * PILL_W],
        _ => [rad, rad],
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn back_geom(
    kind: CursorType,
    prev: CursorType,
    m: f32,
    pos: [f32; 2],
    h: f32,
    sel: Option<([f32; 2], f32)>,
    squash: f32,
    ink: f32,
) -> BackLens {
    let (ha, hb) = (back_of(prev, h), back_of(kind, h));
    let half = [lerp(ha[0], hb[0], m), lerp(ha[1], hb[1], m)];
    let r = half[0].min(half[1]);
    let (mut mn, mut mx) = (
        [pos[0] - half[0], pos[1] - half[1]],
        [pos[0] + half[0], pos[1] + half[1]],
    );
    let text = kind == CursorType::IBeam;
    if let (true, Some((anchor, w))) = (text, sel) {
        let reach = (anchor[0] - pos[0]) * w;
        mn[0] = mn[0].min(pos[0] + reach - half[0]);
        mx[0] = mx[0].max(pos[0] + reach + half[0]);
    }
    BackLens {
        mn,
        mx,
        r,
        squash,
        ink,
        ink_at: pos,
        ring: text,
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
