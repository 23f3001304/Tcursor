// Where cursor packs live on disk. Three sources, and this file is the only place that knows the
// difference: the embedded "default" set (no folder at all), the BUNDLED packs shipped in the app's
// resources, and the user's IMPORTED packs under `cursors_dir()`.
//
// Bundled and imported packs are the same folder shape, so everything downstream (`sprite_sources`,
// `busy_spec`, the preview's `cursor_sprites`) takes a plain `&Path` and never asks which it got.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Folder name, relative to whichever root wins, holding one subfolder per bundled pack.
const BUNDLED_REL: &str = "assets/cursorpacks";

/// The EMBEDDED pack's folder, a sibling of `BUNDLED_REL`. Its bytes are also compiled into the
/// binary (`cursorset::SPRITES`) - which is what the export reads - but the panel's grid needs
/// real files to show, so it ships as a resource too. Its arrow is `pointer.png`; see
/// `packlist::default_filename`.
const DEFAULT_REL: &str = "assets/cursors";

/// Resolved once per process - see `bundled_root` / `default_pack_dir`.
static BUNDLED: OnceLock<Option<PathBuf>> = OnceLock::new();
static DEFAULT_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// `<config-dir>/TCursor/cursors` - one subfolder per IMPORTED pack. Sibling to
/// `settings::store::config_path`'s `TCursor/config.json` (same app-data convention).
pub fn cursors_dir() -> PathBuf {
    dirs_next::config_dir().unwrap_or_else(std::env::temp_dir).join("TCursor").join("cursors")
}

/// Where an imported pack's PNGs + `hotspots.json` + `pack.json` live.
pub fn pack_dir(pack_id: &str) -> PathBuf {
    cursors_dir().join(pack_id)
}

/// Roots that may hold `rel` (an `assets/...` resource folder), most-specific first - the same
/// shape, and the same reasoning, as `win::sys::proc::ffmpeg_candidates`.
///
/// - next to the running executable: the installed build, where `bundle.resources` lands under the
///   install dir;
/// - one and two levels above it: `cargo run` and `tauri dev` both run the exe from
///   `src-tauri/target/<profile>/`, so `../../assets/...` IS the repo's own asset folder - which is
///   why dev needs no staging step and no copy;
/// - last, whatever Tauri itself reports as the resource dir, when a caller has an `AppHandle`
///   to ask (only `list_cursor_packs` does). Belt and braces: the exe-relative entries above
///   already resolve in both dev and an installed build.
pub fn resource_candidates(rel: &str, resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..3 {
            let Some(d) = dir else { break };
            v.push(d.join(rel));
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    if let Some(res) = resource_dir {
        v.push(res.join(rel));
    }
    v
}

/// The first candidate root that exists, memoized for the life of the process.
///
/// The memo is seeded by whichever caller runs first, with or without an `AppHandle`. That is safe
/// because the exe-relative candidates come FIRST and are the ones that actually resolve in both
/// dev and an installed build, so a later call carrying a resource dir cannot disagree with an
/// earlier one that did not.
pub fn bundled_root(resource_dir: Option<&Path>) -> Option<&'static Path> {
    BUNDLED
        .get_or_init(|| resource_candidates(BUNDLED_REL, resource_dir).into_iter().find(|p| p.is_dir()))
        .as_deref()
}

/// The EMBEDDED pack's folder (`assets/cursors`), memoized the same way. `None` when the resource
/// is missing - the pack still lists and still exports, its grid tile just shows no sprite.
pub fn default_pack_dir(resource_dir: Option<&Path>) -> Option<&'static Path> {
    DEFAULT_DIR
        .get_or_init(|| resource_candidates(DEFAULT_REL, resource_dir).into_iter().find(|p| p.is_dir()))
        .as_deref()
}

/// The bundled pack folder for `id`, if there is one. Used both to resolve a selected pack and to
/// keep `pack_import::unique_id` from ever handing an import a bundled pack's id.
pub fn bundled_pack_dir(id: &str) -> Option<PathBuf> {
    let dir = bundled_root(None)?.join(id);
    dir.is_dir().then_some(dir)
}

/// Every bundled pack folder, alphabetical by folder name for a stable grid order.
pub fn bundled_pack_dirs(resource_dir: Option<&Path>) -> Vec<PathBuf> {
    let Some(root) = bundled_root(resource_dir) else { return Vec::new() };
    dirs_in(root)
}

/// Every imported pack folder under `cursors_dir()`, alphabetical.
pub fn imported_pack_dirs() -> Vec<PathBuf> {
    dirs_in(&cursors_dir())
}

/// Subdirectories of `root`, sorted. Empty for a missing or unreadable root.
fn dirs_in(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut dirs: Vec<PathBuf> =
        entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    dirs
}

/// The folder a selected pack id resolves to: a bundled pack first, then an imported one.
///
/// Bundled wins because `list_packs` lists it that way too - one id is one pack in the grid AND in
/// the renderer, with no chance of the preview and the export picking different folders. A pack
/// imported before a bundled one claimed that id is the only casualty; `unique_id` makes sure no
/// NEW import can ever land in that position.
pub fn resolve_pack_dir(pack_id: &str) -> PathBuf {
    bundled_pack_dir(pack_id).unwrap_or_else(|| pack_dir(pack_id))
}

#[cfg(test)]
#[path = "packdirs_tests.rs"]
mod tests;
