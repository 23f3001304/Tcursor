use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::events::track::cursorpixels::CapturedCursor;
use crate::session::paths::ProjectPaths;

/// Upper bound on distinct cursor bitmaps kept for one recording. A take that cycles through
/// more (a custom-cursor game, a theme switcher) keeps the first 64 and reuses the last known
/// shape after that - the layer is a fidelity aid, not a reason to grow the project folder.
pub const MAX_CURSORS: usize = 64;

/// One captured cursor bitmap in `cursor/layer.json`, stored beside it as `cursor/<id>.png`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CursorEntry {
    pub id: u32,
    pub w: u32,
    pub h: u32,
    pub hx: u32,
    pub hy: u32,
    pub file: String,
}

/// The recording's OS-cursor layer: the real cursor bitmaps plus `(t_ms, id)` samples saying
/// which one was showing. Times are paused-aware recording-clock ms, the same stamps
/// `CursorTrack` uses. Its presence is also what tells a recording apart from a pre-layer one
/// whose video has the cursor baked in (see `settings::store::os_cursor_in_video`).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CursorLayer {
    pub cursors: Vec<CursorEntry>,
    pub track: Vec<(u32, u32)>,
}

impl CursorLayer {
    /// Whether this recording captured a cursor layer at all - i.e. whether its video was
    /// captured clean. File existence only: an empty layer still means a clean video.
    pub fn exists(paths: &ProjectPaths) -> bool {
        paths.cursor_layer().exists()
    }

    /// The layer, or `None` for a recording that has none (and for an unreadable/corrupt one -
    /// a missing cursor is never an error, it just falls back to drawing nothing).
    pub fn load(paths: &ProjectPaths) -> Option<Self> {
        std::fs::read(paths.cursor_layer())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }

    /// The cursor id showing at `t_ms` (last sample with `t <= t_ms`), mirroring
    /// `CursorTrack::type_at`. `None` before the first sample, or on an empty track.
    pub fn id_at(&self, t_ms: u32) -> Option<u32> {
        let i = self.track.partition_point(|&(t, _)| t <= t_ms);
        (i > 0).then(|| self.track[i - 1].1)
    }
}

/// Accumulates the layer during a recording: the tracker adds each newly-seen bitmap and marks
/// the moments the shown cursor changes, then `save` writes the JSON + one PNG per entry.
#[derive(Default)]
pub struct CursorLayerBuilder {
    entries: Vec<(CursorEntry, Vec<u8>)>,
    track: Vec<(u32, u32)>,
}

impl CursorLayerBuilder {
    /// True once `MAX_CURSORS` bitmaps are held; the tracker stops capturing new ones.
    pub fn is_full(&self) -> bool {
        self.entries.len() >= MAX_CURSORS
    }

    /// Store `c` and return its id (its index, so ids are dense and stable within a take).
    pub fn add(&mut self, c: CapturedCursor) -> u32 {
        let id = self.entries.len() as u32;
        let entry = CursorEntry { id, w: c.w, h: c.h, hx: c.hx, hy: c.hy, file: format!("{id}.png") };
        self.entries.push((entry, c.rgba));
        id
    }

    /// Record that cursor `id` is the one showing from `t_ms` on.
    pub fn mark(&mut self, t_ms: u32, id: u32) {
        self.track.push((t_ms, id));
    }

    /// The serializable layer (without pixels) - the shape `save` writes and tests assert on.
    pub fn layer(&self) -> CursorLayer {
        CursorLayer {
            cursors: self.entries.iter().map(|(e, _)| e.clone()).collect(),
            track: self.track.clone(),
        }
    }

    /// Write `cursor/layer.json` and one `cursor/<id>.png` per entry. Written even when nothing
    /// was captured: the file's presence is the record that this take's video is cursor-free.
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

/// RGBA8 PNG, using the same `png` encoder the preview already writes frames with.
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
