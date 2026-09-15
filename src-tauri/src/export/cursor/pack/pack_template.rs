use crate::export::cursor::pack::cursorset::SPRITES;
use crate::export::cursor::pack::kind_filename;
use crate::export::cursor::pack::packdirs::bundled_pack_dir;
use std::path::{Path, PathBuf};

const TEMPLATE_SOURCE_ID: &str = "macos-clean";

#[tauri::command]
pub fn create_pack_template(dir: String) -> Result<String, String> {
    let root = PathBuf::from(&dir);
    if !root.is_dir() {
        return Err(format!("not a folder: {dir}"));
    }
    let target = unique_template_dir(&root);
    write_template(&target).map_err(|e| {
        let _ = std::fs::remove_dir_all(&target);
        format!("failed to create pack template: {e}")
    })?;
    Ok(target.to_string_lossy().into_owned())
}

fn unique_template_dir(root: &Path) -> PathBuf {
    let base = root.join("my-pack");
    if !base.exists() {
        return base;
    }
    (2..)
        .map(|n| root.join(format!("my-pack-{n}")))
        .find(|p| !p.exists())
        .unwrap()
}

fn write_template(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let clean_dir = bundled_pack_dir(TEMPLATE_SOURCE_ID);
    for &(kind, builtin_png, _) in SPRITES {
        let filename = kind_filename(kind);
        let bytes = clean_dir
            .as_deref()
            .and_then(|d| std::fs::read(d.join(&filename)).ok())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| builtin_png.to_vec());
        std::fs::write(dir.join(&filename), bytes)?;
    }
    std::fs::write(dir.join("hotspots.json"), HOTSPOTS_JSON)?;
    std::fs::write(dir.join("pack.json"), PACK_JSON)?;
    std::fs::write(dir.join("README.txt"), README_TXT)?;
    Ok(())
}

const HOTSPOTS_JSON: &str = r#"{
  "arrow": [0.2188, 0.0625],
  "ibeam": [0.5, 0.5],
  "hand": [0.4023, 0.0625],
  "resize_ns": [0.5, 0.5],
  "resize_ew": [0.5, 0.5],
  "resize_nwse": [0.4961, 0.5],
  "resize_nesw": [0.5, 0.4961],
  "move": [0.5, 0.5],
  "busy": [0.5, 0.5]
}
"#;

const PACK_JSON: &str = r#"{
  "id": "my-pack",
  "name": "My Pack",
  "category": "Imported",
  "version": 2,
  "busy": {
    "anim": "spin",
    "fps": 24
  }
}
"#;

const README_TXT: &str = include_str!("pack_template_readme.txt");

#[cfg(test)]
#[path = "pack_template_tests.rs"]
mod tests;
