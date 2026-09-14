// Cursor pack resolution: turns a `CursorSettings.pack` id into the (type, PNG bytes, hotspot)
// rows both the export (`cursorset::prep`) and the editor preview (`cursorpreview::cursor_sprites`)
// decode. "default" is the built-in embedded set (`cursorset::SPRITES`); any other id is a pack
// FOLDER - bundled with the app or imported by the user, resolved by `packdirs` - with missing
// kinds falling back to the embedded sprite.
//
// Format v2 (`pack.json` `version: 2`) adds `busy: { anim, fps }`: the pack's busy state animates.
// See `busy.rs` for the poses and `assets/cursorpacks/README.md` for the on-disk format. WHICH packs
// exist, and how the panel's grid shows them, is `packlist.rs`.
use std::collections::HashMap;
use std::path::Path;
use crate::events::track::cursortype::CursorType;
use crate::export::cursor::busy::BusySpec;
use crate::export::cursor::cursorset::SPRITES;
use crate::export::cursor::packdirs::resolve_pack_dir;

/// The built-in pack's id - never a real pack folder name (see `unique_id` in `pack_import.rs`,
/// which never assigns this id to an import).
pub const DEFAULT_PACK_ID: &str = "default";

/// Whether a pack's sprites are RGB-inverted for a dark theme. Only the embedded default set is:
/// it is a monochrome black-on-white arrow drawn to be flipped, so one asset serves both themes.
/// Every bundled or imported pack is artwork with its own colours (an orange cartoon arrow, a blue
/// glass one); inverting those turned them into their negatives on stage and in the export, which
/// is the "cursor colour bug" the owner saw. They are drawn exactly as their PNGs are.
pub fn theme_inverts(pack_id: &str) -> bool { pack_id == DEFAULT_PACK_ID }

/// Upper bound on `busy_NN.png` frames read from one pack, so a stray folder cannot make the
/// renderer hold hundreds of decoded sprites.
pub const MAX_BUSY_FRAMES: u32 = 64;

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

/// `pack.json` as written by `pack_import` (v1: `{id, name}`) or shipped with a bundled pack
/// (v2: `+ version`, `busy`, `category`). Unknown keys - `builtin`, `generated` - are ignored:
/// where a pack came from is decided by WHICH FOLDER it was found in, never by what its own JSON
/// claims. `category` is the STYLE the picker groups by ("Classic", "Playful", ...), which is a
/// claim about the artwork rather than about provenance, so a pack is trusted with its own.
#[derive(serde::Deserialize)]
pub(crate) struct Meta {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) category: Option<String>,
    #[serde(default)]
    pub(crate) busy: Option<BusySpec>,
    /// How the renderer TREATS the sprites, as opposed to what they look like. Absent (the only
    /// state until 2026-09-14) is a plain alpha blit. `"glass"` makes each sprite a LENS: the FX
    /// pass refracts the frame through its alpha mask and the sprite itself lands on top at
    /// `fx_lens::SPRITE_ALPHA`. See `assets/cursorpacks/README.md`.
    #[serde(default)]
    pub(crate) material: Option<String>,
}

pub(crate) fn read_meta(dir: &Path) -> Option<Meta> {
    serde_json::from_slice(&std::fs::read(dir.join("pack.json")).ok()?).ok()
}

/// How many `busy_NN.png` frames `dir` ships, counting from 00 until the first gap.
pub(crate) fn count_busy_frames(dir: &Path) -> u32 {
    (0..MAX_BUSY_FRAMES).take_while(|i| busy_frame_path(dir, *i).is_file()).count() as u32
}

/// `packlist::list_packs` for the pack grid. Takes the `AppHandle` purely to ask Tauri where its
/// resources are; Tauri injects it, so the JS call is unchanged.
///
/// *Why the command lives here rather than beside `list_packs` in `packlist`:* `lib.rs` registers
/// it by path (`export::cursor::pack::list_cursor_packs`), and `tauri::generate_handler!` also
/// needs the macro-generated items that path brings with it - a `pub use` does not carry them.
/// Keeping the one-line wrapper put means the listing split needed no change to the handler list.
#[tauri::command]
pub fn list_cursor_packs(app: tauri::AppHandle) -> Vec<crate::export::cursor::packlist::CursorPackInfo> {
    use tauri::Manager;
    crate::export::cursor::packlist::list_packs(app.path().resource_dir().ok().as_deref())
}

/// The `material` `pack_id` declares, or `None` for the embedded set and any pack that names none
/// (a plain alpha blit). Read from disk, so callers that need it per frame cache the answer -
/// `cursorset::prep` resolves it once into `CursorPrep::glass`.
pub fn material(pack_id: &str) -> Option<String> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID { return None; }
    read_meta(&resolve_pack_dir(pack_id))?.material
}

/// Whether `pack_id`'s sprites are lenses rather than pictures (`material: "glass"`).
pub fn is_glass(pack_id: &str) -> bool {
    material(pack_id).as_deref() == Some(crate::export::fx::fx_lens::GLASS)
}

/// The busy animation `pack_id` declares, or `None` for the embedded set and any v1 pack (whose
/// busy state is a still). `frames` is filled in by counting the folder's `busy_NN.png` files, so
/// an explicit-frame pack overrides its own declared `anim` without saying so twice.
pub fn busy_spec(pack_id: &str) -> Option<BusySpec> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID { return None; }
    let dir = resolve_pack_dir(pack_id);
    let mut spec = read_meta(&dir)?.busy?;
    spec.frames = count_busy_frames(&dir);
    Some(spec)
}

/// Every `busy_NN.png` in `pack_id`'s folder, in order. Empty unless the pack ships explicit
/// frames, in which case they ARE the animation (see `busy::busy_pose`).
pub fn busy_frames(pack_id: &str) -> Vec<Vec<u8>> {
    if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID { return Vec::new(); }
    let dir = resolve_pack_dir(pack_id);
    (0..MAX_BUSY_FRAMES)
        .map(|i| std::fs::read(busy_frame_path(&dir, i)))
        .take_while(|r| r.is_ok())
        .filter_map(|r| r.ok())
        .collect()
}

/// `busy_00.png`, `busy_01.png`, ... - zero-padded to two digits, the format the README documents.
fn busy_frame_path(dir: &Path, i: u32) -> std::path::PathBuf {
    dir.join(format!("busy_{i:02}.png"))
}

/// Resolve a pack id to one `(CursorType, PNG bytes, hotspot)` row per built-in kind - the
/// embedded pack returns `SPRITES` verbatim (Busy excepted, see `busy_as_arrow`); anything else
/// resolves against its folder.
pub fn sprite_sources(pack_id: &str) -> Vec<(CursorType, Vec<u8>, (f32, f32))> {
    let mut rows = if pack_id.is_empty() || pack_id == DEFAULT_PACK_ID {
        SPRITES.iter().map(|&(kind, png, hot)| (kind, png.to_vec(), hot)).collect()
    } else {
        sprite_sources_from_dir(&resolve_pack_dir(pack_id))
    };
    // A pack that ANIMATES its busy state has a busy sprite worth drawing; only a still one gets
    // swapped for the arrow.
    if busy_spec(pack_id).is_none() { busy_as_arrow(&mut rows); }
    rows
}

/// The OS shows "busy" as the plain arrow plus a spinner overlay it draws itself; the embedded
/// set's only busy asset is a static multicolor pinwheel disc, which at cursor size reads as
/// visual corruption rather than "loading" - worse than no animation at all. Rather than patch the
/// asset, resolve Busy to whatever Arrow resolved to, at the single seam both export
/// (`cursorset::prep`) and preview (`cursorpreview::cursor_sprites`) call through. The `Busy`
/// variant itself, and the recorded cursor-type track, are untouched - only which sprite bytes get
/// drawn for it. A v2 pack declaring `busy` opts out: its busy sprite is the animation's frame.
fn busy_as_arrow(rows: &mut [(CursorType, Vec<u8>, (f32, f32))]) {
    let Some(arrow) = rows.iter().find(|(k, ..)| *k == CursorType::Arrow).map(|(_, b, h)| (b.clone(), *h)) else { return };
    if let Some(busy) = rows.iter_mut().find(|(k, ..)| *k == CursorType::Busy) {
        busy.1 = arrow.0;
        busy.2 = arrow.1;
    }
}

/// Pure core of `sprite_sources`, taking the pack folder directly so it is unit-testable against
/// a temp dir (no dependency on the real app-data `cursors_dir()`). A pack overrides any kind
/// whose PNG file is present in `dir` (hotspot from `dir`'s `hotspots.json`, defaulting to a
/// centered `(0.5, 0.5)` if that kind is absent from it), and falls back to the embedded bytes +
/// hotspot for every kind the pack doesn't provide (missing/unreadable file, or an entirely
/// missing/nonexistent `dir`).
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
#[cfg(test)]
#[path = "pack_default_tests.rs"]
mod default_tests;
