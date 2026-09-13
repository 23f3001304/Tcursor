// What cursor packs exist, and how the Cursor panel's grid shows them. Split from `pack.rs`, which
// is now purely "id -> sprite bytes": this file never decodes a pixel, it only names folders and
// files. The one thing both halves share is `pack.json` (`pack::read_meta`).
use std::collections::HashMap;
use std::path::Path;

use crate::events::track::cursortype::CursorType;
use crate::export::cursor::busy::BusySpec;
use crate::export::cursor::cursorset::SPRITES;
use crate::export::cursor::pack::{count_busy_frames, kind_filename, kind_name, read_meta, DEFAULT_PACK_ID};
use crate::export::cursor::packdirs::{bundled_pack_dirs, default_pack_dir, imported_pack_dirs};

/// One selectable cursor pack, as the panel's grid sees it.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    /// Absolute path to the pack's folder, so the grid loads each tile's sprite through the asset
    /// protocol instead of the backend base64ing 135 PNGs into one reply. Empty only when the
    /// folder could not be resolved at all.
    pub dir: String,
    /// Kind wire name -> filename inside `dir`, already alias-resolved and already carrying the
    /// busy-is-arrow substitution. A kind the pack does not provide is simply absent, so the grid
    /// never points an `<img>` at a file that is not there.
    pub files: HashMap<String, String>,
    /// The pack's busy animation, so a hovered tile previews it with the same `busyPose` the
    /// export runs. `None` when the busy state is a still.
    pub busy: Option<BusySpec>,
}

/// The embedded pack's grid row. Its sprites ARE on disk (`assets/cursors`, shipped as a resource)
/// even though the export reads the copies compiled into the binary - the grid cannot show
/// `include_bytes!`.
///
/// *Why no `busy` animation,* even though its `busy.png` is the spinning-beachball disc that would
/// obviously suit one: `pack::busy_as_arrow` substitutes the arrow for Busy on every pack that
/// declares no animation, in the export AND the preview. Declaring one here would make the tile
/// promise a spinning beachball the renderer never draws. See `packlist.md`.
fn embedded() -> CursorPackInfo {
    let dir = default_pack_dir(None);
    CursorPackInfo {
        id: DEFAULT_PACK_ID.to_string(),
        name: "Default".to_string(),
        builtin: true,
        dir: dir.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
        files: dir.map(|d| pack_files(d, true, false)).unwrap_or_default(),
        busy: None,
    }
}

/// The embedded pack's folder predates the pack format and spells its arrow `pointer.png`; every
/// other kind already matches `kind_filename`. One alias, pinned by
/// `packlist_tests::the_default_pack_resolves_all_nine_kinds_through_the_alias_map`.
fn default_filename(kind: CursorType) -> String {
    match kind {
        CursorType::Arrow => "pointer.png".to_string(),
        _ => kind_filename(kind),
    }
}

/// Kind wire name -> the file in `dir` the grid should show for it. Kinds whose file is missing are
/// left out (the renderer falls back to the embedded sprite there, which the grid cannot display).
///
/// `animates` false applies the same busy-is-arrow rule `pack::busy_as_arrow` applies to the bytes,
/// so a tile always shows what picking that pack actually renders - for the embedded pack, for a
/// v1 imported pack, for anything with a still busy state.
fn pack_files(dir: &Path, is_default: bool, animates: bool) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for &(kind, ..) in SPRITES {
        let file = if is_default { default_filename(kind) } else { kind_filename(kind) };
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

/// One pack folder's listing entry. `builtin` comes from the CALLER (the folder it was found in),
/// which is what makes it unspoofable by a `"builtin": true` key in an imported pack's own JSON.
fn read_pack_meta(dir: &Path, builtin: bool) -> Option<CursorPackInfo> {
    let m = read_meta(dir)?;
    let busy = m.busy.map(|mut b| { b.frames = count_busy_frames(dir); b });
    Some(CursorPackInfo {
        id: m.id,
        name: m.name,
        builtin,
        dir: dir.to_string_lossy().into_owned(),
        files: pack_files(dir, false, busy.is_some()),
        busy,
    })
}

/// The embedded set, then every BUNDLED pack, then every IMPORTED one.
///
/// Skips a folder with a missing or unreadable `pack.json`, and any pack whose id is already taken
/// - so one id is exactly one row in the grid, matching `packdirs::resolve_pack_dir`'s
/// bundled-wins rule. `resource_dir` is Tauri's own answer when the caller has an `AppHandle`;
/// `None` relies on the exe-relative candidates, which is what resolves under `tauri dev`.
pub fn list_packs(resource_dir: Option<&Path>) -> Vec<CursorPackInfo> {
    let mut packs = vec![embedded()];
    let found = bundled_pack_dirs(resource_dir).into_iter().map(|d| (d, true))
        .chain(imported_pack_dirs().into_iter().map(|d| (d, false)));
    for (dir, builtin) in found {
        if let Some(info) = read_pack_meta(&dir, builtin) {
            if !packs.iter().any(|p| p.id == info.id) { packs.push(info); }
        }
    }
    packs
}

/// A freshly imported pack's row, built from what `pack_import` just wrote (no `pack.json` read).
pub fn imported_info(id: String, name: String, dir: &Path) -> CursorPackInfo {
    CursorPackInfo {
        id, name, builtin: false,
        dir: dir.to_string_lossy().into_owned(),
        files: pack_files(dir, false, false),
        busy: None,
    }
}

#[cfg(test)]
#[path = "packlist_tests.rs"]
mod tests;
