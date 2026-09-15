use std::collections::HashMap;
use std::path::Path;

use crate::events::track::cursortype::CursorType;
use crate::export::cursor::draw::busy::BusySpec;
use crate::export::cursor::pack::cursorset::SPRITES;
use crate::export::cursor::pack::packdirs::{
    bundled_pack_dirs, default_pack_dir, imported_pack_dirs,
};
use crate::export::cursor::pack::{
    count_busy_frames, kind_filename, kind_name, read_meta, DEFAULT_PACK_ID,
};

pub const IMPORTED_CATEGORY: &str = "Imported";

pub const DEFAULT_PACK_CATEGORY: &str = "Classic";

#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub builtin: bool,
    pub dir: String,
    pub files: HashMap<String, String>,
    pub busy: Option<BusySpec>,
    pub material: Option<String>,
}

fn embedded() -> CursorPackInfo {
    let dir = default_pack_dir(None);
    CursorPackInfo {
        id: DEFAULT_PACK_ID.to_string(),
        name: "Default".to_string(),
        category: DEFAULT_PACK_CATEGORY.to_string(),
        builtin: true,
        dir: dir
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        files: dir.map(|d| pack_files(d, true, false)).unwrap_or_default(),
        busy: None,
        material: None,
    }
}

fn default_filename(kind: CursorType) -> String {
    match kind {
        CursorType::Arrow => "pointer.png".to_string(),
        _ => kind_filename(kind),
    }
}

fn pack_files(dir: &Path, is_default: bool, animates: bool) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for &(kind, ..) in SPRITES {
        let file = if is_default {
            default_filename(kind)
        } else {
            kind_filename(kind)
        };
        if dir.join(&file).is_file() {
            map.insert(kind_name(kind), file);
        }
    }
    if !animates {
        if let Some(arrow) = map.get(&kind_name(CursorType::Arrow)).cloned() {
            map.insert(kind_name(CursorType::Busy), arrow);
        }
    }
    map
}

fn read_pack_meta(dir: &Path, builtin: bool) -> Option<CursorPackInfo> {
    let m = read_meta(dir)?;
    let busy = m.busy.map(|mut b| {
        b.frames = count_busy_frames(dir);
        b
    });
    Some(CursorPackInfo {
        id: m.id,
        name: m.name,
        category: category_or_imported(m.category),
        builtin,
        dir: dir.to_string_lossy().into_owned(),
        files: pack_files(dir, false, busy.is_some()),
        busy,
        material: m.material,
    })
}

fn category_or_imported(raw: Option<String>) -> String {
    raw.map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| IMPORTED_CATEGORY.to_string())
}

pub fn list_packs(resource_dir: Option<&Path>) -> Vec<CursorPackInfo> {
    let mut packs = vec![embedded()];
    let found = bundled_pack_dirs(resource_dir)
        .into_iter()
        .map(|d| (d, true))
        .chain(imported_pack_dirs().into_iter().map(|d| (d, false)));
    for (dir, builtin) in found {
        if let Some(info) = read_pack_meta(&dir, builtin) {
            if !packs.iter().any(|p| p.id == info.id) {
                packs.push(info);
            }
        }
    }
    packs
}

pub fn imported_info(id: String, name: String, dir: &Path) -> CursorPackInfo {
    CursorPackInfo {
        id,
        name,
        category: IMPORTED_CATEGORY.to_string(),
        builtin: false,
        dir: dir.to_string_lossy().into_owned(),
        files: pack_files(dir, false, false),
        busy: None,
        material: None,
    }
}

#[cfg(test)]
#[path = "packlist_tests.rs"]
mod tests;
