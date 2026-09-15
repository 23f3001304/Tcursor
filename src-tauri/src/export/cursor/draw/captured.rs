use std::collections::HashMap;

use crate::events::track::cursorlayer::CursorLayer;
use crate::export::cursor::draw::cursordraw::{draw_cursor, CursorSprite};
use crate::export::cursor::pack::cursorset::frame_placement;
use crate::export::scene::Panel;
use crate::export::types::{Camera, FramePoint};
use crate::session::paths::ProjectPaths;
use crate::settings::cursor::CursorStyle;

pub fn draws_captured(style: CursorStyle, has_layer: bool) -> bool {
    style == CursorStyle::System && has_layer
}

pub fn content_scale(panel: f32, inset_w: f32, sw: u32) -> f32 {
    panel * inset_w / sw.max(1) as f32
}

pub struct CapturedCursors {
    sprites: HashMap<u32, CursorSprite>,
    layer: CursorLayer,
}

impl CapturedCursors {
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

    pub fn sprite_at(&self, t_ms: u32) -> Option<&CursorSprite> {
        self.layer.id_at(t_ms).and_then(|id| self.sprites.get(&id))
    }

    pub fn draw(
        &self,
        out: &mut [u8],
        ow: u32,
        oh: u32,
        cur: FramePoint,
        cam: Camera,
        screen: &Panel,
        inset_w: f32,
        sw: u32,
        ev_t: u32,
    ) {
        let Some((pos, panel, clip)) = frame_placement(cur, cam, ow, oh, screen, inset_w) else {
            return;
        };
        let Some(spr) = self.sprite_at(ev_t) else {
            return;
        };
        let s = content_scale(panel, inset_w, sw);
        draw_cursor(
            out,
            ow,
            oh,
            spr,
            pos,
            &[],
            spr.canvas_h as f32 * s,
            0.0,
            1.0,
            clip,
        );
    }
}

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

fn decode_rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(
        png::Transformations::normalize_to_color8() | png::Transformations::ALPHA,
    );
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
