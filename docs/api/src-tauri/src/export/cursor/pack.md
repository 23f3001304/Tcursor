# src-tauri/src/export/cursor/pack.rs

Cursor pack resolution. Turns a `CursorSettings.pack` id into the `(CursorType, PNG bytes, hotspot)` rows both the export (`cursorset::prep`) and the editor preview (`cursorpreview::cursor_sprites`) decode. `"default"` is the built-in embedded set (`cursorset::SPRITES`); any other id is an imported pack folder under `cursors_dir()`, falling back to the built-in sprite for any kind the pack doesn't provide. Importing a pack is a separate concern, in `pack_import.rs`.

## DEFAULT_PACK_ID

```rust
pub const DEFAULT_PACK_ID: &str = "default";
```

The built-in pack's id. Never assigned to a real imported pack folder (`pack_import::unique_id` only ever produces slugs derived from a user-chosen folder name, and `"default"` is reserved).

### Used by

- `src-tauri/src/settings/model.rs` (`CursorSettings::default`) - the persisted default value for `pack`
- `sprite_sources`, `list_cursor_packs` (this file) - the built-in-vs-imported branch point

## CursorPackInfo

```rust
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
}
```

One selectable cursor pack, as returned to the frontend.

- `id: String` - persists into `CursorSettings.pack`. `"default"` for the built-in set; a slug (e.g. `"my_pack"`, `"my_pack_2"`) for an imported one.
- `name: String` - display name shown in the picker. For an imported pack, the source folder's original name (case/spacing preserved).
- `builtin: bool` - `true` only for the embedded default (not stored on disk).

### Used by

- `list_cursor_packs` (this file), `pack_import::import_cursor_pack` - both produce this DTO
- `src/lib/ipc.ts` (`CursorPackInfo`) - the TS mirror
- `src/editor/panels/CursorPanel.tsx` - renders the pack picker from a `CursorPackInfo[]`

## cursors_dir

```rust
pub fn cursors_dir() -> PathBuf
```

`<config-dir>/TCursor/cursors` - one subfolder per imported pack. Sibling to `settings::store::config_path`'s `TCursor/config.json` (same app-data convention: `dirs_next::config_dir()`, falling back to a temp dir).

## pack_dir

```rust
pub fn pack_dir(pack_id: &str) -> PathBuf
```

`cursors_dir().join(pack_id)` - where an imported pack's PNGs, `hotspots.json`, and `pack.json` live.

## kind_name

```rust
pub fn kind_name(kind: CursorType) -> String
```

A `CursorType`'s wire name (`"arrow"`, `"resize_ns"`, ...), via a serde round-trip (`serde_json::to_value` then read the string) rather than a second hand-maintained table - mirrors `edit::seed::layout_name`'s trick, so the name can never drift from `CursorType`'s own `#[serde(rename)]` attributes.

### Returns

The lowercase serde name for `kind`, or `""` if serialization ever fails (not reachable in practice - `CursorType` always serializes to a string).

### Behaviors

- `kind_name_round_trips_every_variant_through_serde` - every `CursorType` in `cursorset::SPRITES` produces a non-empty name, and `kind_filename` is exactly `"{name}.png"`.

## kind_filename

```rust
pub fn kind_filename(kind: CursorType) -> String
```

Expected on-disk filename for a cursor kind in a pack folder: `"{kind_name(kind)}.png"`, e.g. `resize_ns.png`. This is the shared basis both `sprite_sources_from_dir` (reading a pack) and `pack_import::collect_valid_sprites` (importing one) use, so they can never disagree on what a pack folder should contain.

### Behaviors

- `kind_filename_matches_serde_name_plus_png` - `Arrow` -> `"arrow.png"`, `ResizeNs` -> `"resize_ns.png"`.

## list_cursor_packs

```rust
#[tauri::command]
pub fn list_cursor_packs() -> Vec<CursorPackInfo>
```

Every selectable pack: the built-in default first, then every imported pack folder under `cursors_dir()`.

### Returns

`Vec<CursorPackInfo>` - always starts with `{id: "default", name: "Default", builtin: true}`. Then, for each subfolder of `cursors_dir()` (alphabetical by folder name), its `pack.json` metadata if present and parseable; folders with no/unreadable `pack.json` are silently skipped (never partially imported by anything other than `pack_import`, but tolerant of manual tampering).

### Implementation

1. Start `packs` with the built-in entry.
2. `std::fs::read_dir(cursors_dir())`; on any error (folder doesn't exist yet - no packs imported), skip straight to returning just the built-in entry.
3. Collect subfolder paths, sort them (stable order across calls), and read each one's `pack.json` via `read_pack_meta`, keeping only the ones that parse.

### Behaviors

- `list_cursor_packs_always_includes_the_builtin_default_first` - `packs[0]` is always `{id: "default", builtin: true}`, even with no imported packs on disk.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`
- `src/lib/ipc.ts` (`listCursorPacks`) - the TS wrapper
- `src/editor/panels/CursorPanel.tsx` - calls on mount to populate the picker

## sprite_sources

```rust
pub fn sprite_sources(pack_id: &str) -> Vec<(CursorType, Vec<u8>, (f32, f32))>
```

Resolve a pack id to one `(CursorType, PNG bytes, hotspot)` row per built-in kind - the single function both the export and the preview call to turn "which pack is selected" into decodable sprite bytes.

### Inputs

- `pack_id: &str` - `CursorSettings.pack`. *Why a string, not an enum:* imported packs are discovered at runtime; the valid id set isn't known at compile time.

### Returns

One row per entry in `cursorset::SPRITES`, in the same order. For `pack_id == "default"` (or `""`), every row is the built-in `(kind, png.to_vec(), hot)`. Otherwise, delegates to `sprite_sources_from_dir(&pack_dir(pack_id))`. Either way, `busy_as_arrow` then overwrites the `Busy` row's bytes/hotspot with `Arrow`'s before returning.

### Behaviors

- `default_pack_id_resolves_to_builtin_sprites_verbatim_except_busy` / `empty_pack_id_also_resolves_to_builtin` - fast-path to `SPRITES`, Busy excepted.
- `busy_resolves_to_arrow_bytes_and_hotspot_for_the_builtin_pack` - Busy's row is Arrow's bytes/hotspot, not the builtin pinwheel PNG.
- `busy_resolves_to_arrow_even_when_a_custom_pack_overrides_arrow` - a custom pack's own Arrow override wins for Busy too (the remap happens after per-kind resolution, not before).

### Used by

- `src-tauri/src/export/cursor/cursorset.rs` (`prep`) - resolves export sprite bytes
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`) - resolves preview sprite bytes

## busy_as_arrow

```rust
fn busy_as_arrow(rows: &mut [(CursorType, Vec<u8>, (f32, f32))])
```

The OS shows "busy" as the plain arrow plus a spinner it draws itself; this app's only busy asset is a static multicolor pinwheel disc, which at cursor size (dimmed preview or baked into an export) reads as visual corruption rather than "loading". Rather than replace the asset, overwrites the `Busy` row in `rows` with whatever `Arrow` resolved to - built-in or a custom pack's own override - so both export and preview draw a plain arrow for Busy. No-op if `rows` has no `Arrow` entry (not reachable via `sprite_sources`, since `SPRITES` always includes one). The `CursorType::Busy` variant and the recorded cursor-type track are untouched - only which sprite bytes get drawn for it.

### Used by

- `sprite_sources` (this file) - applied after either resolution branch

## sprite_sources_from_dir

```rust
fn sprite_sources_from_dir(dir: &Path) -> Vec<(CursorType, Vec<u8>, (f32, f32))>
```

Pure core of `sprite_sources`: resolve every built-in kind against a specific pack folder. Not `pub` - taking `dir` directly (rather than resolving it internally from a pack id via the real `cursors_dir()`) is what makes this unit-testable against a temp folder instead of the real app-data directory.

### Inputs

- `dir: &Path` - the pack folder to read from. Reading a nonexistent `dir` is not an error - every kind simply falls back to the built-in (all lookups become `None`).

### Returns

One row per `cursorset::SPRITES` entry. For each kind: if `dir/{kind_filename(kind)}` exists, is readable, and is non-empty, that pack's bytes are used, with the hotspot from `dir/hotspots.json` (defaulting to `(0.5, 0.5)` if that kind is absent from it). Otherwise the built-in bytes + hotspot for that kind.

### Implementation

1. `read_hotspots(dir/hotspots.json)` once (not per-kind).
2. For each `(kind, builtin_png, builtin_hot)` in `SPRITES`: try `std::fs::read(dir/kind_filename(kind))`, filtering out an empty result (treated the same as absent); on success look up the hotspot by `kind_name(kind)`, else fall back to `(builtin_png, builtin_hot)`.

### Behaviors

- `nonexistent_pack_folder_falls_back_to_builtin_for_every_kind` - a `dir` that doesn't exist resolves identically to the built-in pack.
- `custom_png_overrides_one_kind_others_fall_back_to_builtin` - a pack providing only `arrow.png` + a matching `hotspots.json` entry overrides just `Arrow`; `Hand` (and every other kind) still resolves to the built-in bytes/hotspot.
- `missing_hotspot_entry_for_a_provided_kind_defaults_to_center` - a provided PNG with no corresponding `hotspots.json` entry gets `(0.5, 0.5)`.
- `empty_png_file_is_treated_as_absent_and_falls_back` - a zero-byte override file is ignored, not used as a (broken) sprite.

## read_hotspots

```rust
fn read_hotspots(path: &Path) -> HashMap<String, (f32, f32)>
```

`hotspots.json` -> `{kind_name: (hx, hy)}`. Defaults to an empty map on any error (missing file, bad JSON) so a pack with no/partial hotspot data still resolves - just with centered hotspots for the kinds it doesn't specify.

## read_pack_meta

```rust
fn read_pack_meta(dir: &Path) -> Option<CursorPackInfo>
```

One pack folder's metadata: reads and parses `dir/pack.json` (written by `pack_import::write_pack` as `{"id", "name"}`). `None` if the file is missing or fails to parse - `list_cursor_packs` skips such folders rather than erroring the whole list.
