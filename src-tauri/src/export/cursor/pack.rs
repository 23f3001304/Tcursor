// Cursor pack resolution: turns a `CursorSettings.pack` id into the (type, PNG bytes, hotspot)
// rows both the export (`cursorset::prep`) and the editor preview (`cursorpreview::cursor_sprites`)
// decode. "default" is the built-in embedded set (`cursorset::SPRITES`); any other id is an
// imported pack folder under `cursors_dir()`, missing kinds falling back to the built-in sprite.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::events::track::cursortype::CursorType;
use crate::export::cursor::cursorset::SPRITES;

/// The built-in pack's id - never a real imported-pack folder name (see `unique_id` in
/// `pack_import.rs`, which never assigns this id to an import).
pub const DEFAULT_PACK_ID: &str = "default";

/// One selectable cursor pack: `id` persists into `CursorSettings.pack`, `name` is shown in the
/// picker, `builtin` distinguishes the embedded set (not stored on disk) from an imported one.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
}

/// `<config-dir>/TCursor/cursors` - one subfolder per imported pack. Sibling to
/// `settings::store::config_path`'s `TCursor/config.json` (same app-data convention).
pub fn cursors_dir() -> PathBuf {
    dirs_next::config_dir().unwrap_or_else(std::env::temp_dir).join("TCursor").join("cursors")
}

/// Where an imported pack's PNGs + `hotspots.json` + `pack.json` live.
pub fn pack_dir(pack_id: &str) -> PathBuf {
    cursors_dir().join(pack_id)
}

/// A `CursorType`'s wire name (`"arrow"`, `"resize_ns"`, ...) - the shared basis for both the
/// expected PNG filename (`{name}.png`) and the `hotspots.json` key, so the two can never drift
/// from `CursorType`'s own `#[serde(rename)]` table. Mirrors `edit::seed::layout_name`'s trick.
pub fn kind_name(kind: CursorType) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

/// Expected on-disk filename for a cursor kind in a pack folder, e.g. `resize_ns.png`.
pub fn kind_filename(kind: CursorType) -> String { format!("{}.png", kind_name(kind)) }

/// `hotspots.json` -> `{kind_name: (hx, hy)}`, defaulting to an empty map on any error (missing
/// file, bad JSON) so a pack with no/partial hotspot data still imports - just centered hotspots.
fn read_hotspots(path: &Path) -> HashMap<String, (f32, f32)> {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// One pack folder's metadata (`pack.json` = `{"id", "name"}`, written by `pack_import`).
fn read_pack_meta(dir: &Path) -> Option<CursorPackInfo> {
    #[derive(serde::Deserialize)]
    struct Meta { id: String, name: String }
    let bytes = std::fs::read(dir.join("pack.json")).ok()?;
    let m: Meta = serde_json::from_slice(&bytes).ok()?;
    Some(CursorPackInfo { id: m.id, name: m.name, builtin: false })
}

/// Built-in pack first, then every imported pack folder under `cursors_dir()` (alphabetical by
/// folder name for a stable order), skipping any folder missing/with unreadable `pack.json`.
#[tauri::command]
pub fn list_cursor_packs() -> Vec<CursorPackInfo> {
    let mut packs = vec![CursorPackInfo { id: DEFAULT_PACK_ID.to_string(), name: "Default".to_string(), builtin: true }];
    if let Ok(entries) = std::fs::read_dir(cursors_dir()) {
        let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort();
        packs.extend(dirs.iter().filter_map(|d| read_pack_meta(d)));
    }
    packs
}

/// Resolve a pack id to one `(CursorType, PNG bytes, hotspot)` row per built-in kind - the
/// built-in pack returns `SPRITES` verbatim; anything else resolves against its folder.
pub fn sprite_sources(pack_id: &str) -> Vec<(CursorType, Vec<u8>, (f32, f32))> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        return SPRITES.iter().map(|&(kind, png, hot)| (kind, png.to_vec(), hot)).collect();
    }
    sprite_sources_from_dir(&pack_dir(pack_id))
}

/// Pure core of `sprite_sources`, taking the pack folder directly so it is unit-testable against
/// a temp dir (no dependency on the real app-data `cursors_dir()`). An imported pack overrides
/// any kind whose PNG file is present in `dir` (hotspot from `dir`'s `hotspots.json`, defaulting
/// to a centered `(0.5, 0.5)` if that kind is absent from it), and falls back to the built-in
/// bytes + hotspot for every kind the pack doesn't provide (missing/unreadable file, or an
/// entirely missing/nonexistent `dir`).
fn sprite_sources_from_dir(dir: &Path) -> Vec<(CursorType, Vec<u8>, (f32, f32))> {
    let hotspots = read_hotspots(&dir.join("hotspots.json"));
    SPRITES.iter().map(|&(kind, builtin_png, builtin_hot)| {
        match std::fs::read(dir.join(kind_filename(kind))).ok().filter(|b| !b.is_empty()) {
            Some(bytes) => {
                let hot = hotspots.get(&kind_name(kind)).copied().unwrap_or((0.5, 0.5));
                (kind, bytes, hot)
            }
            None => (kind, builtin_png.to_vec(), builtin_hot),
        }
    }).collect()
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod tests;
