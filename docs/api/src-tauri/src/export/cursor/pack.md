# src-tauri/src/export/cursor/pack.rs

Cursor pack resolution - "id to sprite bytes". WHICH packs exist, and how the panel's grid shows them, is `packlist.rs`. Turns a `CursorSettings.pack` id into the `(CursorType, PNG bytes, hotspot)` rows both the export (`cursorset::prep`) and the editor preview (`cursorpreview::cursor_sprites`) decode. `"default"` is the built-in embedded set (`cursorset::SPRITES`) - and stays so even now that its sprites ALSO ship as a folder for the grid to show; any other id is a pack folder, bundled or imported, resolved by `packdirs` and falling back to the embedded sprite for any kind the pack doesn't provide. Importing a pack is a separate concern, in `pack_import.rs`.

## theme_inverts

```rust
pub fn theme_inverts(pack_id: &str) -> bool
```

True only for the embedded default set, a monochrome arrow drawn to be RGB-flipped for a dark theme so one asset serves both. Every bundled or imported pack is artwork with its own colours and is drawn exactly as its PNGs are; inverting them used to turn an orange cartoon arrow into a blue one on stage and in the export (the owner's "cursor colour bug", 2026-09-13). `cursorset::prep` and `cursorpreview::cursor_sprites` both gate their invert on it.

## DEFAULT_PACK_ID

```rust
pub const DEFAULT_PACK_ID: &str = "default";
```

The built-in pack's id. Never assigned to a real imported pack folder (`pack_import::unique_id` only ever produces slugs derived from a user-chosen folder name, and `"default"` is reserved).

### Used by

- `src-tauri/src/settings/model.rs` (`CursorSettings::default`) - the persisted default value for `pack`
- `sprite_sources`, `list_cursor_packs` (this file) - the built-in-vs-imported branch point

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
pub fn list_cursor_packs(app: tauri::AppHandle) -> Vec<packlist::CursorPackInfo>
```

`packlist::list_packs` for the pack grid. Takes the `AppHandle` purely to ask Tauri where its resources are; Tauri injects it, so the JS call is unchanged.

*Why this one-line wrapper lives here rather than beside `list_packs` in `packlist`:* `lib.rs` registers it by path (`export::cursor::pack::list_cursor_packs`), and `tauri::generate_handler!` also needs the macro-generated items that path brings with it - a `pub use` does not carry them. Keeping it put meant the listing split needed no change to the handler list.

## busy_spec

```rust
pub fn busy_spec(pack_id: &str) -> Option<BusySpec>
```

The busy animation `pack_id` declares, or `None` for the embedded set and any v1 pack (whose busy state is a still).

`frames` is filled in by `count_busy_frames`, not read from `pack.json` - so a pack that ships explicit `busy_NN.png` files overrides its own declared `anim` without having to say so twice.

Also the gate on `busy_as_arrow`: a pack that ANIMATES its busy state keeps its own busy sprite.

## busy_frames

```rust
pub fn busy_frames(pack_id: &str) -> Vec<Vec<u8>>
```

Every `busy_NN.png` in `pack_id`'s folder, in order, stopping at the first gap. Empty unless the pack ships explicit frames, in which case they ARE the animation (see `busy::busy_pose`). None of the fifteen shipped packs does; the path exists because the format allows it.

## count_busy_frames

```rust
pub(crate) fn count_busy_frames(dir: &Path) -> u32
```

How many `busy_NN.png` frames `dir` ships, counting from 00 until the first gap, capped at `MAX_BUSY_FRAMES`.

## busy_frame_path

```rust
fn busy_frame_path(dir: &Path, i: u32) -> std::path::PathBuf
```

`busy_00.png`, `busy_01.png`, ... - zero-padded to two digits, the format `assets/cursorpacks/README.md` documents. The one place that filename is spelled.

## MAX_BUSY_FRAMES

```rust
pub const MAX_BUSY_FRAMES: u32 = 64
```

Upper bound on `busy_NN.png` frames read from one pack, so a stray folder cannot make the renderer hold hundreds of decoded sprites.


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

- `src-tauri/src/export/cursor/pack/cursorset.rs` (`prep`) - resolves export sprite bytes
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`) - resolves preview sprite bytes

### Behaviors

- `the_default_pack_still_exports_from_the_embedded_sprites_not_its_folder` - **the export must not change.** `"default"` having a real folder (`assets/cursors`, for the panel's grid) is a listing concern only: this function still returns `SPRITES` verbatim with busy remapped to arrow, never the folder's files. Pinned twice - the bytes and hotspot of all nine rows, and one actually drawn frame compared against the same draw from the embedded bytes - so a later "just resolve default through its folder" refactor cannot slip past.

## busy_as_arrow

```rust
fn busy_as_arrow(rows: &mut [(CursorType, Vec<u8>, (f32, f32))])
```

The OS shows "busy" as the plain arrow plus a spinner it draws itself; this app's only busy asset is a static multicolor pinwheel disc, which at cursor size (dimmed preview or baked into an export) reads as visual corruption rather than "loading". Rather than replace the asset, overwrites the `Busy` row in `rows` with whatever `Arrow` resolved to - built-in or a custom pack's own override - so both export and preview draw a plain arrow for Busy. No-op if `rows` has no `Arrow` entry (not reachable via `sprite_sources`, since `SPRITES` always includes one). The `CursorType::Busy` variant and the recorded cursor-type track are untouched - only which sprite bytes get drawn for it.

### Used by

- `sprite_sources` (this file) - applied after either resolution branch

*Opt-out:* a v2 pack that declares `busy` never goes through this - `sprite_sources` only calls it when `busy_spec` is `None`. Its busy sprite is the animation's own frame, so swapping in the arrow would throw away the thing the pack exists to show.

## sprite_sources_from_dir

```rust
pub(crate) fn sprite_sources_from_dir(dir: &Path) -> Vec<(CursorType, Vec<u8>, (f32, f32))>
```

Pure core of `sprite_sources`: resolve every built-in kind against a specific pack folder. `pub(crate)` (not exported past this crate) - taking `dir` directly, rather than resolving it internally from a pack id via the real `cursors_dir()`, is what makes this unit-testable against a temp folder instead of the real app-data directory. `pack_template.rs`'s own test reuses it for the same reason: it is the real loader, run against the just-written template folder.

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

## Meta

```rust
pub(crate) struct Meta { id: String, name: String, category: Option<String>, busy: Option<BusySpec> }
```

`pack.json` as written by `pack_import` (v1: `{id, name}`) or shipped with a bundled pack (v2: `+ version`, `busy`, `category`). Unknown keys - `builtin`, `generated`, `version` itself - are ignored: where a pack came from is decided by which folder it was found in, never by what its own JSON claims, and the presence of `busy` is a better version signal than the number beside it.

`category` is the STYLE the editor's picker groups the pack by, and it IS trusted to the pack's own JSON - unlike `builtin` - because it is a claim about the artwork rather than about provenance. It is `Option` here and resolved to a real string by `packlist::category_or_imported`, so a v1 manifest (every pack a user imports) needs no migration. `assets/cursorpacks/README.md` documents the field for whoever adds the next pack.

### Meta::material

```rust
#[serde(default)]
pub(crate) material: Option<String>,
```

How the renderer TREATS a pack's sprites, as opposed to what they depict. Absent - the only state until 2026-09-14, and still the state of every pack but one - is a plain alpha blit. `"glass"` makes each sprite a LENS: the FX pass refracts the recorded frame through its silhouette and the sprite's own pixels are then blitted at `fx_lens::SPRITE_ALPHA`.

Documented for pack authors in `assets/cursorpacks/README.md`, which also states the consequence: artwork for a glass pack should be a CLEAR lens with highlights and a rim, because whatever it paints opaquely is frame the lens cannot bend.

## read_meta

```rust
pub(crate) fn read_meta(dir: &Path) -> Option<Meta>
```

Parse `dir/pack.json`, or `None` on any error. `pub(crate)` because `packlist` reads the same file for its own half of the job (id, name, category, busy) while this file reads it for resolution.

## material

```rust
pub fn material(pack_id: &str) -> Option<String>
```

The `material` `pack_id` declares, or `None` for the embedded set and any pack that names none.

Reads `pack.json` off disk, so a caller that needs it per frame caches the answer - `cursorset::prep` resolves it once into `CursorPrep::glass`, and `packlist::read_pack_meta` carries it to the frontend on `CursorPackInfo`.

## is_glass

```rust
pub fn is_glass(pack_id: &str) -> bool
```

Whether `pack_id`'s sprites are lenses rather than pictures (`material: "glass"`). Same disk read as `material`; same caching rule.

### Used by

- `src-tauri/src/export/cursor/pack/cursorset.rs` - `prep`, once per renderer build, to set `CursorPrep::glass` and decide whether to build the lens masks at all.

## cursorset

Submodule (`cursor/pack/cursorset.rs`). Manages the per-type cursor sprite set: decodes each shape once at prep time, inverts RGB for dark themes, and dispatches per-frame draw calls with panel-proportional sizing. Key items: `SPRITES`, `CursorPrep`, `prep`, `sprite_for`, `posed` (the busy animation's per-frame sprite + transform), `draw`, `frame_placement` (the projection both cursor paths share), `invert_rgb` - full per-symbol docs in `cursor/pack/cursorset.md`.

## pack_import

Submodule (`cursor/pack/pack_import.rs`). Import a user-chosen folder as a new cursor pack: validates it holds at least one recognized sprite, copies the recognized files under `packdirs::cursors_dir()`, and hands back the `CursorPackInfo` the panel can select at once. Key item: `import_cursor_pack` (Tauri command) - full per-symbol docs in `cursor/pack/pack_import.md`.

## pack_template

Submodule (`cursor/pack/pack_template.rs`). M8(c) "Create pack template": writes a fresh, complete pack folder (nine sprites seeded from the bundled Clean pack, a v2 `pack.json` with a declared busy animation, `hotspots.json`, `README.txt`) so a pack author has a real working folder to start from. Key item: `create_pack_template` (Tauri command) - full per-symbol docs in `cursor/pack/pack_template.md`.

## packdirs

Submodule (`cursor/pack/packdirs.rs`). Where cursor packs live: the embedded set, the BUNDLED folders under the app's `assets/cursorpacks` resources, and the user's imports under `cursors_dir()`. Resolves a pack id to a folder (bundled wins) and explains why the exe-relative candidates make `tauri dev` work with no staging step. Key items: `cursors_dir`, `pack_dir`, `resource_candidates`, `bundled_root`, `default_pack_dir` (the embedded pack's own `assets/cursors` folder), `bundled_pack_dir`, `bundled_pack_dirs`, `imported_pack_dirs`, `resolve_pack_dir` - full per-symbol docs in `cursor/pack/packdirs.md`.

## packlist

Submodule (`cursor/pack/packlist.rs`). What packs exist and how the Cursor panel's grid shows them - names, folders, and a kind-to-filename map that already carries the embedded pack's `pointer.png` alias and the busy-is-arrow substitution, so the frontend needs neither rule. Key items: `CursorPackInfo`, `list_packs`, `imported_info`, `pack_files`, `default_filename` - full per-symbol docs in `cursor/pack/packlist.md`.
