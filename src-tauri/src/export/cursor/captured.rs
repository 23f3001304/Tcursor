// The REAL OS cursor, composited from the layer the recorder captured (`events::track::
// cursorlayer`). This is what "System" means on a recording made since the capture went
// cursor-free: the actual bitmap that was on screen, at the raw recorded point, with none of the
// Enhanced polish (no glide, no idealization, no bounce, no trail). Enhanced still goes through
// `cursorset`; Hidden still draws nothing.
use std::collections::HashMap;

use crate::events::track::cursorlayer::CursorLayer;
use crate::export::cursor::cursordraw::{draw_cursor, CursorSprite};
use crate::export::cursor::cursorset::frame_placement;
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint};
use crate::session::paths::ProjectPaths;
use crate::settings::cursor::CursorStyle;

/// Whether this frame draws the captured cursor instead of the synthetic one. The style is read
/// LIVE from the doc (switching it in the editor takes effect in the warm preview immediately);
/// `has_layer` is a property of the recording. A `System` recording with NO layer is a pre-layer
/// one, where `cursorset`'s plain arrow stays the only thing that can be drawn.
pub fn draws_captured(style: CursorStyle, has_layer: bool) -> bool {
    style == CursorStyle::System && has_layer
}

/// Output pixels per SOURCE pixel for this frame's screen panel - the scale that keeps the
/// captured cursor at its true size RELATIVE TO THE SCREEN CONTENT. The captured bitmap is in
/// source pixels, and the source is drawn into the screen panel, so one source pixel is
/// `screen.rect.w / sw` output pixels; written as the shared `panel` factor times the inset's own
/// source->output ratio, which is the same number (`panel * inset_w == screen.rect.w`) with the
/// panel shrink applied exactly ONCE.
///
/// Without this a 4K take exported at 1080p drew the cursor at its captured 4K pixel size, about
/// twice its on-screen proportion. Same-resolution takes are unaffected (`inset_w == sw` gives 1).
pub fn content_scale(panel: f32, inset_w: f32, sw: u32) -> f32 {
    panel * inset_w / sw.max(1) as f32
}

/// The recording's cursor layer, decoded once per renderer: every captured bitmap as a sprite,
/// plus the timeline saying which was showing when.
pub struct CapturedCursors {
    sprites: HashMap<u32, CursorSprite>,
    layer: CursorLayer,
}

impl CapturedCursors {
    /// Load and decode the layer, or `None` for a recording that has none / whose PNGs are all
    /// unreadable. Never an error: a missing layer just means the synthetic path stays in charge.
    pub fn load(paths: &ProjectPaths) -> Option<Self> {
        let layer = CursorLayer::load(paths)?;
        let mut sprites = HashMap::new();
        for e in &layer.cursors {
            let bytes = std::fs::read(paths.cursor_dir().join(&e.file)).ok();
            if let Some(spr) = bytes.and_then(|b| sprite(&b, e.hx, e.hy)) {
                sprites.insert(e.id, spr);
            }
        }
        (!sprites.is_empty()).then_some(Self { sprites, layer })
    }

    /// The bitmap showing at event time `t_ms`, or `None` before the first sample.
    pub fn sprite_at(&self, t_ms: u32) -> Option<&CursorSprite> {
        self.layer.id_at(t_ms).and_then(|id| self.sprites.get(&id))
    }

    /// Per-frame draw, taking the same arguments as `cursorset::draw` (plus the source width) so
    /// `composite_at` can pick between them. The sprite lands with its hotspot on the raw recorded
    /// point, at `content_scale` - the real cursor's size relative to the screen content.
    pub fn draw(&self, out: &mut [u8], ow: u32, oh: u32, cur: FramePoint, cam: Camera,
                screen: &Panel, inset_w: f32, sw: u32, ev_t: u32) {
        let Some((pos, panel, clip)) = frame_placement(cur, cam, ow, oh, screen, inset_w) else { return };
        let Some(spr) = self.sprite_at(ev_t) else { return };
        // `draw_cursor` derives `scale = size_px * bounce / canvas_h`; `canvas_h` is the sprite's
        // own height, so asking for `canvas_h * s` pixels of height IS a scale of `s`.
        // No trail (empty `recent`, blur 0) and no bounce - this is the real cursor, unidealized.
        let s = content_scale(panel, inset_w, sw);
        draw_cursor(out, ow, oh, spr, pos, &[], spr.canvas_h as f32 * s, 0.0, 1.0, clip);
    }
}

/// Decode one captured PNG into the BGRA sprite the blit wants, with the hotspot re-expressed as
/// the 0..1 fraction `draw_cursor` multiplies by the scaled size.
fn sprite(png: &[u8], hx: u32, hy: u32) -> Option<CursorSprite> {
    let (rgba, w, h) = decode_rgba(png)?;
    let mut bgra = rgba;
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    Some(CursorSprite {
        bgra,
        w,
        h,
        hot: (hx as f32 / w.max(1) as f32, hy as f32 / h.max(1) as f32),
        canvas_h: h,
    })
}

/// RGBA8 pixels of a PNG, normalizing paletted/16-bit/no-alpha inputs the way `brand_icon` does.
fn decode_rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some((buf, info.width, info.height))
}

#[cfg(test)]
#[path = "captured_tests.rs"]
mod tests;
