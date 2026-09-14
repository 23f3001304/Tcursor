# src-tauri/src/export/cursor/packlist.rs

What cursor packs exist, and how the Cursor panel's grid shows them. Split from `pack.rs`, which is now purely "id to sprite bytes": this file never decodes a pixel, it only names folders and files. The one thing both halves share is `pack.json` (`pack::read_meta`).

**The embedded pack is a real folder now.** `assets/cursors` ships as a resource alongside `assets/cursorpacks`, so "Default" lists with a `dir` and a `files` map exactly like the fifteen bundled packs, and its grid tile shows a sprite and cycles on hover. What it does NOT change is the export: `pack::sprite_sources("default")` still returns `cursorset::SPRITES` - the bytes compiled into the binary - and never reads that folder. Pinned by `pack_tests::the_default_pack_still_exports_from_the_embedded_sprites_not_its_folder`, which checks both the bytes and one actually drawn frame.

## IMPORTED_CATEGORY

```rust
pub const IMPORTED_CATEGORY: &str = "Imported"
```

The category a pack that names none of its own is listed under. A bundled pack always states one - `packlist_tests::every_bundled_pack_manifest_declares_a_known_category` fails the build otherwise - so in practice this is the user's own imports, whose `pack.json` is written by `pack_import` in the v1 shape and has no category key at all.

## DEFAULT_PACK_CATEGORY

```rust
pub const DEFAULT_PACK_CATEGORY: &str = "Classic"
```

The category the embedded set is listed under. It is the plain system arrow, which is exactly what "Classic" means in the picker. Stated here rather than in a manifest because the embedded set is the one pack with no `pack.json` of its own.

## CursorPackInfo

```rust
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub builtin: bool,
    pub dir: String,
    pub files: HashMap<String, String>,
    pub busy: Option<BusySpec>,
}
```

One selectable cursor pack, as the panel's grid sees it.

- `id` - persists into `CursorSettings.pack`; `"default"` is the embedded set.
- `name` - shown on the tile, from the pack's own `pack.json` (hardcoded for the embedded one).
- `category` - the STYLE section the editor's picker lists this pack under (`Classic`, `Glass and glow`, `Playful`, `Drawn`, `Retro`, `Imported`). Never empty. *Why this one IS trusted to the pack's own JSON, when `builtin` is not:* a category is a claim about the pack's ARTWORK, which is the pack's to make; `builtin` is a claim about provenance, which only the folder can settle. The frontend therefore holds no list of pack ids at all (`packCategories.md`), and adding a pack to a section is a one-line edit in `assets/cursorpacks/<pack>/pack.json`.
- `builtin` - the user cannot delete it: the embedded set, or a BUNDLED folder. Decided by which folder the pack was found in (`packdirs`), never by what its own `pack.json` claims.
- `dir` - absolute path to the pack's folder, so the grid loads each tile's sprite through the asset protocol. *Why a path and not data URLs:* base64ing 15 packs x 9 PNGs into one reply is roughly a megabyte, and each sprite would have to go through `decode_sprite` - which shells out to ffmpeg - so opening the Cursor panel would launch 135 subprocesses.
- `files` - kind wire name to filename inside `dir`. Already alias-resolved and already carrying the busy-is-arrow substitution, so the grid needs to know neither rule. A kind the pack does not ship is simply absent, so the grid never points an `<img>` at a file that is not there.
- `busy` - the pack's busy animation, so a hovered tile previews it with the same `busyPose` the export runs. `None` when the busy state is a still.

### CursorPackInfo::material

```rust
pub material: Option<String>,
```

The pack's `material` (`"glass"`, or `None` for a plain alpha blit) - how the renderer TREATS the sprites, not what they depict. Comes from the pack's own `pack.json` via `pack::read_meta`, like `category` and for the same reason: it is a claim about the ARTWORK, which a pack is trusted with, rather than about provenance, which it is not.

On the wire so the picker can say a pack refracts the frame rather than leaving the user to discover it in the export. `pack_tests::exactly_one_shipped_pack_is_a_lens_and_the_default_is_never_one` pins that this listing and `pack::material` never disagree.

## embedded

```rust
fn embedded() -> CursorPackInfo
```

The embedded pack's grid row. Its sprites ARE on disk (`assets/cursors`, shipped as a resource) even though the export reads the copies compiled into the binary - the grid cannot show `include_bytes!`. A missing resource degrades to `dir: ""` and no `files`: the pack still lists and still exports, its tile just shows the name alone.

### Why it declares no busy animation

Its `busy.png` is the spinning-beachball disc, which obviously *would* suit one - and the brief that added the folder asked for `spin` at 24 fps. It does not get one, because `pack::busy_as_arrow` substitutes the arrow for Busy on every pack that declares no animation, in the **export and the preview** alike. Declaring an animation here would make the tile promise a spinning beachball that picking the pack never draws, and removing the substitution would change exported frames - which the same brief forbade, and pinned with a byte-identity test.

So the two rules are kept consistent instead: no declared animation, and `pack_files` maps the Busy slot to the arrow so the tile shows exactly what the renderer draws. Animating it everywhere is a one-line gate change (`sprite_sources`' `busy_spec(pack_id).is_none()` check) whenever that trade is worth making.

## default_filename

```rust
fn default_filename(kind: CursorType) -> String
```

The embedded pack's folder predates the pack format and spells its arrow `pointer.png`; every other kind already matches `kind_filename` (`hand`, `ibeam`, `move`, `busy`, and the four `resize_*`). One alias, and the test asserts both halves of that claim - the alias itself, and that no other kind needs one.

## pack_files

```rust
fn pack_files(dir: &Path, is_default: bool, animates: bool) -> HashMap<String, String>
```

Kind wire name to the file in `dir` the grid should show for it.

- Kinds whose file is missing are left out. The renderer falls back to the embedded sprite there, which the grid cannot display, so the tile skips that state rather than loading a 404.
- `is_default` picks `default_filename` over `kind_filename` - the alias map.
- `animates` false applies the same busy-is-arrow rule `pack::busy_as_arrow` applies to the bytes. That keeps a tile honest for the embedded pack, for a v1 imported pack, and for anything else with a still busy state.

## read_pack_meta

```rust
fn read_pack_meta(dir: &Path, builtin: bool) -> Option<CursorPackInfo>
```

One pack folder's listing entry: `pack::read_meta` plus the folder path, its frame-counted `BusySpec`, and its `files` map. `builtin` comes from the CALLER (the folder it was found in), which is what makes it unspoofable by a `"builtin": true` key in an imported pack's own JSON. The category comes from the JSON, through `category_or_imported`.

## category_or_imported

```rust
fn category_or_imported(raw: Option<String>) -> String
```

A `pack.json` category, or `IMPORTED_CATEGORY` when it is absent or only whitespace - the picker must never be handed a nameless section, and a blank string is exactly as useless as no key at all. Surrounding whitespace is trimmed, so `"Playful "` joins the Playful section rather than opening a second one beside it.

## list_packs

```rust
pub fn list_packs(resource_dir: Option<&Path>) -> Vec<CursorPackInfo>
```

The embedded set, then every BUNDLED pack, then every IMPORTED one.

Skips a folder with a missing or unreadable `pack.json`, and **any pack whose id is already taken** - so one id is exactly one row in the grid, matching `packdirs::resolve_pack_dir`'s bundled-wins rule.

`resource_dir` is Tauri's own answer when the caller has an `AppHandle`; `None` relies on the exe-relative candidates, which is what resolves under `tauri dev` and in the test binary. Split from the command (`pack::list_cursor_packs`) so tests can call it without a Tauri app.

### Behaviors

- `the_embedded_default_pack_lists_first_with_a_real_folder` - `"default"` leads, named "Default", `builtin`, and its `dir` is a real directory.
- `the_default_pack_resolves_all_nine_kinds_through_the_alias_map` - every mapped filename exists on disk; arrow maps to `pointer.png`; no other kind needs an alias.
- `a_still_busy_state_shows_the_arrow_in_the_grid_exactly_as_it_renders` - the embedded pack's Busy entry equals its Arrow entry.
- `a_v2_pack_keeps_its_own_busy_file_and_its_animation` - "cat" keeps `busy.png` and its `Pulse` spec.
- `a_kind_the_pack_does_not_ship_is_left_out_rather_than_pointing_at_nothing`.
- `list_packs_puts_the_embedded_default_first_then_every_bundled_pack`, `one_id_is_one_row_even_if_an_import_shares_a_bundled_name`.
- `the_embedded_default_pack_is_listed_under_classic`.
- `every_bundled_pack_manifest_declares_a_known_category` - reads the repo's own `assets/cursorpacks` (via `CARGO_MANIFEST_DIR`, not the resolved resource root) so it pins the manifests that actually ship, and refuses any category outside the six the picker curates - `Imported` included, since a bundled pack landing on the user's own shelf is exactly the wrong answer.
- `a_pack_with_no_category_of_its_own_lists_under_imported`, `a_freshly_imported_pack_gets_the_same_category_a_relisting_would_give_it`.

## imported_info

```rust
pub fn imported_info(id: String, name: String, dir: &Path) -> CursorPackInfo
```

A freshly imported pack's row, built from what `pack_import::write_pack` just wrote - no `pack.json` read, since the caller already knows the id and name it chose. An import is always v1 (`busy: None`), so its Busy slot maps to its arrow like any other still pack, and its category is `IMPORTED_CATEGORY` - which is also exactly what re-listing that folder from disk will produce, so the tile does not move the next time the panel opens.

### Used by

- `src-tauri/src/export/cursor/pack.rs` - `list_cursor_packs` wraps `list_packs`
- `src-tauri/src/export/cursor/pack_import.rs` - `import_cursor_pack` returns `imported_info`
- `src/editor/panels/CursorPackGrid.tsx` - the whole grid is driven by `CursorPackInfo`

