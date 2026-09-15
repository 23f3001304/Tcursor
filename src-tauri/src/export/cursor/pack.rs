pub mod cursorset;
pub mod pack_import;
pub mod pack_template;
pub mod packdirs;
pub mod packlist;

use crate::events::track::cursortype::CursorType;
use crate::export::cursor::draw::busy::BusySpec;
use crate::export::cursor::pack::cursorset::SPRITES;
use crate::export::cursor::pack::packdirs::resolve_pack_dir;
use std::collections::HashMap;
use std::path::Path;

pub const DEFAULT_PACK_ID: &str = "default";

pub fn theme_inverts(pack_id: &str) -> bool {
    pack_id == DEFAULT_PACK_ID
}

pub const MAX_BUSY_FRAMES: u32 = 64;

pub fn kind_name(kind: CursorType) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn kind_filename(kind: CursorType) -> String {
    format!("{}.png", kind_name(kind))
}

fn read_hotspots(path: &Path) -> HashMap<String, (f32, f32)> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[derive(serde::Deserialize)]
pub(crate) struct Meta {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) category: Option<String>,
    #[serde(default)]
    pub(crate) busy: Option<BusySpec>,
    #[serde(default)]
    pub(crate) material: Option<String>,
}

pub(crate) fn read_meta(dir: &Path) -> Option<Meta> {
    serde_json::from_slice(&std::fs::read(dir.join("pack.json")).ok()?).ok()
}

pub(crate) fn count_busy_frames(dir: &Path) -> u32 {
    (0..MAX_BUSY_FRAMES)
        .take_while(|i| busy_frame_path(dir, *i).is_file())
        .count() as u32
}

#[tauri::command]
pub fn list_cursor_packs(
    app: tauri::AppHandle,
) -> Vec<crate::export::cursor::pack::packlist::CursorPackInfo> {
    use tauri::Manager;
    crate::export::cursor::pack::packlist::list_packs(app.path().resource_dir().ok().as_deref())
}

pub fn material(pack_id: &str) -> Option<String> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        return None;
    }
    read_meta(&resolve_pack_dir(pack_id))?.material
}

pub fn is_glass(pack_id: &str) -> bool {
    material(pack_id).as_deref() == Some(crate::export::fx::fx_lens::GLASS)
}

pub fn busy_spec(pack_id: &str) -> Option<BusySpec> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        return None;
    }
    let dir = resolve_pack_dir(pack_id);
    let mut spec = read_meta(&dir)?.busy?;
    spec.frames = count_busy_frames(&dir);
    Some(spec)
}

pub fn busy_frames(pack_id: &str) -> Vec<Vec<u8>> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        return Vec::new();
    }
    let dir = resolve_pack_dir(pack_id);
    (0..MAX_BUSY_FRAMES)
        .map(|i| std::fs::read(busy_frame_path(&dir, i)))
        .take_while(|r| r.is_ok())
        .filter_map(|r| r.ok())
        .collect()
}

fn busy_frame_path(dir: &Path, i: u32) -> std::path::PathBuf {
    dir.join(format!("busy_{i:02}.png"))
}

pub fn sprite_sources(pack_id: &str) -> Vec<(CursorType, Vec<u8>, (f32, f32))> {
    let mut rows = if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        SPRITES
            .iter()
            .map(|&(kind, png, hot)| (kind, png.to_vec(), hot))
            .collect()
    } else {
        sprite_sources_from_dir(&resolve_pack_dir(pack_id))
    };
    if busy_spec(pack_id).is_none() {
        busy_as_arrow(&mut rows);
    }
    rows
}

fn busy_as_arrow(rows: &mut [(CursorType, Vec<u8>, (f32, f32))]) {
    let Some(arrow) = rows
        .iter()
        .find(|(k, ..)| *k == CursorType::Arrow)
        .map(|(_, b, h)| (b.clone(), *h))
    else {
        return;
    };
    if let Some(busy) = rows.iter_mut().find(|(k, ..)| *k == CursorType::Busy) {
        busy.1 = arrow.0;
        busy.2 = arrow.1;
    }
}

pub(crate) fn sprite_sources_from_dir(dir: &Path) -> Vec<(CursorType, Vec<u8>, (f32, f32))> {
    let hotspots = read_hotspots(&dir.join("hotspots.json"));
    SPRITES
        .iter()
        .map(|&(kind, builtin_png, builtin_hot)| {
            match std::fs::read(dir.join(kind_filename(kind)))
                .ok()
                .filter(|b| !b.is_empty())
            {
                Some(bytes) => {
                    let hot = hotspots
                        .get(&kind_name(kind))
                        .copied()
                        .unwrap_or((0.5, 0.5));
                    (kind, bytes, hot)
                }
                None => (kind, builtin_png.to_vec(), builtin_hot),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "pack/pack_tests.rs"]
mod tests;
