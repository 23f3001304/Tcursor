// Import a user-chosen folder (arrow.png/ibeam.png/.../hotspots.json - see pack.rs's format doc)
// as a new cursor pack: validate it has at least one recognized sprite, copy it under
// `pack::cursors_dir()`, and hand back the `CursorPackInfo` the picker can select immediately.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::export::cursor::cursorset::SPRITES;
use crate::export::cursor::pack::{kind_filename, pack_dir, CursorPackInfo};
use crate::export::pipeline::ffio::png_dims;

/// Validate `path` is a folder containing at least one recognized cursor PNG, then copy the
/// recognized PNGs + `hotspots.json` (if present and valid) into a new `cursors_dir()` subfolder.
#[tauri::command]
pub fn import_cursor_pack(path: String) -> Result<CursorPackInfo, String> {
    let src = PathBuf::from(&path);
    if !src.is_dir() {
        return Err(format!("not a folder: {path}"));
    }
    let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("Cursor Pack").to_string();
    let sprites = collect_valid_sprites(&src);
    if sprites.is_empty() {
        return Err(format!(
            "no recognized cursor PNGs found in \"{name}\" (expected files like arrow.png, hand.png, ibeam.png, ...)"
        ));
    }
    let hotspots_json = read_and_validate_hotspots(&src)?;
    let id = unique_id(&slugify(&name));
    write_pack(&pack_dir(&id), &id, &name, &sprites, hotspots_json.as_deref())
        .map_err(|e| { let _ = std::fs::remove_dir_all(pack_dir(&id)); format!("failed to import pack: {e}") })?;
    Ok(CursorPackInfo { id, name, builtin: false })
}

/// Every `(filename, bytes)` in `src` that both names a recognized cursor kind and decodes as a
/// PNG with real (nonzero) dimensions. Kinds the folder doesn't provide (or provides invalid
/// files for) are silently skipped - the pack still imports, `pack::sprite_sources` fills the
/// gap with the built-in sprite for those kinds.
fn collect_valid_sprites(src: &Path) -> Vec<(String, Vec<u8>)> {
    SPRITES.iter().filter_map(|&(kind, ..)| {
        let filename = kind_filename(kind);
        let bytes = std::fs::read(src.join(&filename)).ok()?;
        png_dims(&bytes)?;
        Some((filename, bytes))
    }).collect()
}

/// `hotspots.json` in `src`, if present: `Ok(Some(raw bytes))` when it parses as a
/// `{kind_name: [hx, hy]}` map, `Ok(None)` when the file is simply absent (every hotspot then
/// defaults to a centered `(0.5, 0.5)` per `pack::sprite_sources`), `Err` when present but not
/// valid JSON in that shape - a real authoring mistake worth surfacing rather than discarding.
fn read_and_validate_hotspots(src: &Path) -> Result<Option<Vec<u8>>, String> {
    let path = src.join("hotspots.json");
    let Ok(bytes) = std::fs::read(&path) else { return Ok(None) };
    serde_json::from_slice::<HashMap<String, (f32, f32)>>(&bytes)
        .map_err(|e| format!("hotspots.json is not valid (\"kind\": [hx, hy]) JSON: {e}"))?;
    Ok(Some(bytes))
}

/// Lowercase alnum-only slug for a pack id seed (`"My Pack!"` -> `"my_pack"`), never empty.
fn slugify(name: &str) -> String {
    let s: String = name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    match s.trim_matches('_') {
        "" => "pack".to_string(),
        trimmed => trimmed.to_string(),
    }
}

/// `base`, or the first of `base_2`, `base_3`, ... not already present under `cursors_dir()`.
fn unique_id(base: &str) -> String {
    if !pack_dir(base).exists() {
        return base.to_string();
    }
    (2..).map(|n| format!("{base}_{n}")).find(|id| !pack_dir(id).exists()).unwrap()
}

/// Write the new pack folder at `dir`: each recognized sprite PNG, `hotspots.json` (the source
/// file verbatim, or `{}` if it had none), and `pack.json` (`{id, name}`, read back by
/// `pack::list_cursor_packs`). Takes `dir` directly (rather than deriving it from `id`) so it is
/// unit-testable against a temp folder.
fn write_pack(dir: &Path, id: &str, name: &str, sprites: &[(String, Vec<u8>)], hotspots_json: Option<&[u8]>) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (filename, bytes) in sprites {
        std::fs::write(dir.join(filename), bytes)?;
    }
    std::fs::write(dir.join("hotspots.json"), hotspots_json.unwrap_or(b"{}"))?;
    let meta = serde_json::json!({ "id": id, "name": name });
    std::fs::write(dir.join("pack.json"), serde_json::to_vec_pretty(&meta)?)?;
    Ok(())
}

#[cfg(test)]
#[path = "pack_import_tests.rs"]
mod tests;
