use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::events::track::cursorpixels::CapturedCursor;
use crate::events::track::steady::{steady, SHAPE_HOLD_MS};
use crate::session::paths::ProjectPaths;

pub const MAX_CURSORS: usize = 64;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CursorEntry {
    pub id: u32,
    pub w: u32,
    pub h: u32,
    pub hx: u32,
    pub hy: u32,
    pub file: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CursorLayer {
    pub cursors: Vec<CursorEntry>,
    pub track: Vec<(u32, u32)>,
}

impl CursorLayer {
    pub fn exists(paths: &ProjectPaths) -> bool {
        paths.cursor_layer().exists()
    }

    pub fn load(paths: &ProjectPaths) -> Option<Self> {
        let raw: Self = std::fs::read(paths.cursor_layer())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())?;
        Some(Self {
            track: steady(&raw.track, SHAPE_HOLD_MS),
            ..raw
        })
    }

    pub fn id_at(&self, t_ms: u32) -> Option<u32> {
        let i = self.track.partition_point(|&(t, _)| t <= t_ms);
        (i > 0).then(|| self.track[i - 1].1)
    }
}

#[derive(Default)]
pub struct CursorLayerBuilder {
    entries: Vec<(CursorEntry, Vec<u8>)>,
    track: Vec<(u32, u32)>,
}

impl CursorLayerBuilder {
    pub fn is_full(&self) -> bool {
        self.entries.len() >= MAX_CURSORS
    }

    pub fn add(&mut self, c: CapturedCursor) -> u32 {
        let id = self.entries.len() as u32;
        let entry = CursorEntry {
            id,
            w: c.w,
            h: c.h,
            hx: c.hx,
            hy: c.hy,
            file: format!("{id}.png"),
        };
        self.entries.push((entry, c.rgba));
        id
    }

    pub fn mark(&mut self, t_ms: u32, id: u32) {
        self.track.push((t_ms, id));
    }

    pub fn layer(&self) -> CursorLayer {
        CursorLayer {
            cursors: self.entries.iter().map(|(e, _)| e.clone()).collect(),
            track: self.track.clone(),
        }
    }

    pub fn save(&self, paths: &ProjectPaths) -> std::io::Result<()> {
        std::fs::create_dir_all(paths.cursor_dir())?;
        for (e, rgba) in &self.entries {
            write_png(&paths.cursor_png(e.id), rgba, e.w, e.h)?;
        }
        let json = serde_json::to_vec(&self.layer())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(paths.cursor_layer(), json)
    }
}

fn write_png(path: &Path, rgba: &[u8], w: u32, h: u32) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let io = |e: png::EncodingError| std::io::Error::new(std::io::ErrorKind::Other, e);
    let mut writer = enc.write_header().map_err(io)?;
    writer.write_image_data(rgba).map_err(io)
}

#[cfg(test)]
#[path = "cursorlayer_tests.rs"]
mod tests;
