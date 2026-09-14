// The glass cursor material: the refracting LENS the FX pass runs where the cursor sprite is about
// to land, plus the optional "cursor back" (a glass shape BEHIND any pack's cursor). Both are
// described here in output pixels and rendered by `fx_lens.wgsl` (GPU) / `fx_lensdraw.rs` (CPU);
// `fx_lensbuild.rs` is what places them for one frame.
//
// Everything here is pure math on an event-clock timestamp plus the recorded tracks, so one instant
// always yields one shape - the same determinism `busy.rs` keeps, and what lets a paused preview
// show exactly the frame the export would write.
use std::sync::Arc;
use crate::events::model::{Button, EventKind, MouseEvent};
use crate::events::track::cursortype::{CursorTrack, CursorType};

/// The `pack.json` `material` value that turns a pack's sprites into lenses.
pub const GLASS: &str = "glass";
/// How much of a glass pack's own sprite is blitted over the live refraction: its baked highlights
/// and rim are the "glass edge", the frame bending underneath is the body.
pub const SPRITE_ALPHA: f32 = 0.65;
/// The magnification inside a glass shape: a pixel shows what lies `1/ZOOM` of its distance from
/// the shape's centre, so everything under the glass is uniformly this much bigger - and readable.
/// The number `fx_lens.wgsl::LENS_ZOOM` and `cursorGlass.ts::LENS_ZOOM` carry; `fx_lensdraw.rs`
/// reads it from here.
pub const ZOOM: f32 = 1.35;
/// The back's diameter as a multiple of the sprite's drawn height.
pub const BACK_SCALE: f32 = 2.2;
/// The back's height over text, as a fraction of its width - the disc becomes a HORIZONTAL pill,
/// lying along the line of text the way Crystal's own I-beam sprite and a selection highlight do.
pub const PILL_W: f32 = 0.35;
/// Peak click squash, and how long it takes to come back.
pub const SQUASH: f32 = 0.92;
pub const SQUASH_MS: f32 = 120.0;
/// How long the click ink-drop takes to spread through the mask.
pub const INK_MS: f32 = 260.0;
/// How long a shape change (arrow -> pill, pill -> selection) eases over.
pub const MORPH_MS: f32 = 160.0;

pub use crate::export::fx::fx_lensmask::{mask_of, morph_mask, LensMask};

/// The sprite-shaped lens: the sprite's placed box (centre + size, OUTPUT px), the busy rotation in
/// radians, the click squash (1.0 at rest) and the ink drop's progress (negative = none) from
/// `ink_at`.
#[derive(Clone, Debug, PartialEq)]
pub struct CursorLens {
    pub cbox: [f32; 4], pub angle: f32,
    pub squash: f32, pub ink: f32, pub ink_at: [f32; 2],
    pub mask: Arc<LensMask>,
}

/// The cursor back: one rounded rect, which is a disc (`r` = half its height), a vertical pill over
/// text, or a selection bar stretched to the mouse-down point - three shapes the morph can lerp
/// between because they are the same primitive. `ring` is the over-text click look (a line hugging
/// the pill) in place of the ink drop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackLens {
    pub mn: [f32; 2], pub mx: [f32; 2], pub r: f32,
    pub squash: f32, pub ink: f32, pub ink_at: [f32; 2], pub ring: bool,
}

/// Both glass shapes for one frame. Either half may be absent: a glass pack with no back, a plain
/// pack with one, or both together.
#[derive(Clone, Debug, PartialEq)]
pub struct Lenses { pub glass: Option<CursorLens>, pub back: Option<BackLens> }

/// One left-button press and the release that ended it (`u32::MAX` while still held), pre-extracted
/// at prep time so the per-frame lookup does not re-scan the whole event log.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSpan { pub down: u32, pub up: u32, pub x: i32, pub y: i32 }

/// The click effects' own ease-out cubic, reused so a morph and a ripple started at the same
/// instant move together (`clickfx::ease_out`, mirrored by `fx_clicks.wgsl::fx_ease`).
fn ease(p: f32) -> f32 { crate::export::fx::clickfx::ease_out(p) }

/// How long ago the most recent click at or before `t` was, in ms.
fn since_click(clicks: &[u32], t: u32) -> Option<f32> {
    clicks.iter().rev().find(|&&c| c <= t).map(|&c| (t - c) as f32)
}

/// The lens scale at event time `t`: `SQUASH` at the instant of a click, back to 1.0 over
/// `SQUASH_MS`.
pub fn squash_at(clicks: &[u32], t: u32) -> f32 {
    match since_click(clicks, t) {
        Some(dt) if dt < SQUASH_MS => SQUASH + (1.0 - SQUASH) * (dt / SQUASH_MS),
        _ => 1.0,
    }
}

/// The ink drop's progress 0..1 at event time `t`, or a negative number when no drop is live.
pub fn ink_at(clicks: &[u32], t: u32) -> f32 {
    match since_click(clicks, t) {
        Some(dt) if dt < INK_MS => dt / INK_MS,
        _ => -1.0,
    }
}

/// Every left-button press in `events`, paired with its release. Presses with no release yet (the
/// button is still down at the end of the recording) get `u32::MAX`.
pub fn drag_spans(events: &[MouseEvent]) -> Vec<DragSpan> {
    let mut out: Vec<DragSpan> = Vec::new();
    for e in events {
        if e.button != Some(Button::Left) { continue; }
        match e.kind {
            EventKind::Down => out.push(DragSpan { down: e.t, up: u32::MAX, x: e.x, y: e.y }),
            EventKind::Up => { if let Some(last) = out.last_mut() { if last.up == u32::MAX { last.up = e.t; } } }
            EventKind::Move => {}
        }
    }
    out
}

/// The text-selection anchor at event time `t`: the desktop point the left button went down at, and
/// how far the stretch has eased in. 1.0 while held, easing back to 0 over `MORPH_MS` after the
/// release, `None` once that is over - so the bar retracts instead of vanishing.
pub fn selection_at(spans: &[DragSpan], t: u32) -> Option<([i32; 2], f32)> {
    let i = spans.partition_point(|s| s.down <= t);
    let s = spans.get(i.checked_sub(1)?)?;
    let p = |ms: f32| ease((ms / MORPH_MS).clamp(0.0, 1.0));
    if s.up > t { return Some(([s.x, s.y], p((t - s.down) as f32))); }
    let after = (t - s.up) as f32;
    (after < MORPH_MS).then(|| ([s.x, s.y], 1.0 - p(after)))
}

/// The cursor kind at event time `t`, the one before it, and how far the change has eased through
/// `MORPH_MS`. `1.0` (settled) for an empty track or a sample older than the morph.
pub fn kind_morph(track: &CursorTrack, t: u32) -> (CursorType, CursorType, f32) {
    let i = track.samples.partition_point(|&(s, _)| s <= t);
    if i == 0 { return (CursorType::Arrow, CursorType::Arrow, 1.0); }
    let (at, kind) = track.samples[i - 1];
    let prev = if i >= 2 { track.samples[i - 2].1 } else { CursorType::Arrow };
    (kind, prev, ease(((t.saturating_sub(at)) as f32 / MORPH_MS).clamp(0.0, 1.0)))
}

/// The back's half-extents for one cursor kind, about a cursor at the origin: a disc for every
/// shape but the I-beam, which is a HORIZONTAL pill `PILL_W` as tall - a line of text is horizontal,
/// and so is the selection bar it stretches into. `h` is the sprite's drawn height in output px.
/// The corner radius is always the shorter half-extent, which is what makes one rounded rect serve
/// as circle, pill and bar.
fn back_of(kind: CursorType, h: f32) -> [f32; 2] {
    let rad = h * BACK_SCALE * 0.5;
    match kind {
        CursorType::IBeam => [rad, rad * PILL_W],
        _ => [rad, rad],
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// The back for one frame: `kind`'s shape morphed `m` of the way from `prev`'s, centred on the
/// cursor point `pos`, and - over text with the left button held - stretched into a selection bar
/// reaching `sel`'s anchor point (already in output px). Every shape is the same rounded rect, so
/// the morph is a lerp of two corners and a radius rather than a special case per pair.
pub fn back_geom(kind: CursorType, prev: CursorType, m: f32, pos: [f32; 2], h: f32,
                 sel: Option<([f32; 2], f32)>, squash: f32, ink: f32) -> BackLens {
    let (ha, hb) = (back_of(prev, h), back_of(kind, h));
    let half = [lerp(ha[0], hb[0], m), lerp(ha[1], hb[1], m)];
    let r = half[0].min(half[1]);
    let (mut mn, mut mx) = ([pos[0] - half[0], pos[1] - half[1]], [pos[0] + half[0], pos[1] + half[1]]);
    // A selection bar keeps the pill's height and its rounded ends; only the x extent grows, to
    // wherever the press started. `w` eases the stretch in and back out (see `selection_at`).
    let text = kind == CursorType::IBeam;
    if let (true, Some((anchor, w))) = (text, sel) {
        let reach = (anchor[0] - pos[0]) * w;
        mn[0] = mn[0].min(pos[0] + reach - half[0]);
        mx[0] = mx[0].max(pos[0] + reach + half[0]);
    }
    BackLens { mn, mx, r, squash, ink, ink_at: pos, ring: text }
}

#[cfg(test)]
#[path = "fx_lens_tests.rs"]
mod tests;
