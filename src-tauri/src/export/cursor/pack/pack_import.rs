use crate::export::cursor::pack::cursorset::SPRITES;
use crate::export::cursor::pack::kind_filename;
use crate::export::cursor::pack::packdirs::{bundled_pack_dir, pack_dir};
use crate::export::cursor::pack::packlist::{imported_info, CursorPackInfo};
use crate::export::pipeline::ffio::png_dims;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn import_cursor_pack(path: String) -> Result<CursorPackInfo, String> {
    let src = PathBuf::from(&path);
    if !src.is_dir() {
        return Err(format!("not a folder: {path}"));
    }
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Cursor Pack")
        .to_string();
    let sprites = collect_valid_sprites(&src);
    if sprites.is_empty() {
        return Err(format!(
            "no recognized cursor PNGs found in \"{name}\" (expected files like arrow.png, hand.png, ibeam.png, ...)"
        ));
    }
    let hotspots_json = read_and_validate_hotspots(&src)?;
    let id = unique_id(&slugify(&name));
    write_pack(
        &pack_dir(&id),
        &id,
        &name,
        &sprites,
        hotspots_json.as_deref(),
    )
    .map_err(|e| {
        let _ = std::fs::remove_dir_all(pack_dir(&id));
        format!("failed to import pack: {e}")
    })?;
    Ok(imported_info(id.clone(), name, &pack_dir(&id)))
}

fn collect_valid_sprites(src: &Path) -> Vec<(String, Vec<u8>)> {
    SPRITES
        .iter()
        .filter_map(|&(kind, ..)| {
            let filename = kind_filename(kind);
            let bytes = std::fs::read(src.join(&filename)).ok()?;
            png_dims(&bytes)?;
            Some((filename, bytes))
        })
        .collect()
}

fn read_and_validate_hotspots(src: &Path) -> Result<Option<Vec<u8>>, String> {
    let path = src.join("hotspots.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(None);
    };
    serde_json::from_slice::<HashMap<String, (f32, f32)>>(&bytes)
        .map_err(|e| format!("hotspots.json is not valid (\"kind\": [hx, hy]) JSON: {e}"))?;
    Ok(Some(bytes))
}

fn slugify(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    match s.trim_matches('_') {
        "" => "pack".to_string(),
        trimmed => trimmed.to_string(),
    }
}

fn unique_id(base: &str) -> String {
    let taken = |id: &str| pack_dir(id).exists() || bundled_pack_dir(id).is_some();
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base}_{n}"))
        .find(|id| !taken(id))
        .unwrap()
}

fn write_pack(
    dir: &Path,
    id: &str,
    name: &str,
    sprites: &[(String, Vec<u8>)],
    hotspots_json: Option<&[u8]>,
) -> std::io::Result<()> {
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
