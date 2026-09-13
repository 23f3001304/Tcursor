# src-tauri/src/export/cursor/packlist.rs

What cursor packs exist, and how the Cursor panel's grid shows them. Split from `pack.rs`, which is now purely "id to sprite bytes": this file never decodes a pixel, it only names folders and files. The one thing both halves share is `pack.json` (`pack::read_meta`).

**The embedded pack is a real folder now.** `assets/cursors` ships as a resource alongside `assets/cursorpacks`, so "Default" lists with a `dir` and a `files` map exactly like the fifteen bundled packs, and its grid tile shows a sprite and cycles on hover. What it does NOT change is the export: `pack::sprite_sources("default")` still returns `cursorset::SPRITES` - the bytes compiled into the binary - and never reads that folder. Pinned by `pack_tests::the_default_pack_still_exports_from_the_embedded_sprites_not_its_folder`, which checks both the bytes and one actually drawn frame.

## CursorPackInfo

```rust
pub struct CursorPackInfo {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    pub dir: String,
    pub files: HashMap<String, String>,
    pub busy: Option<BusySpec>,
}
```

One selectable cursor pack, as the panel's grid sees it.

- `id` - persists into `CursorSettings.pack`; `"default"` is the embedded set.
- `name` - shown on the tile, from the pack's own `pack.json` (hardcoded for the embedded one).
- `builtin` - the user cannot delete it: the embedded set, or a BUNDLED folder. Decided by which folder the pack was found in (`packdirs`), never by what its own `pack.json` claims.
- `dir` - absolute path to the pack's folder, so the grid loads each tile's sprite through the asset protocol. *Why a path and not data URLs:* base64ing 15 packs x 9 PNGs into one reply is roughly a megabyte, and each sprite would have to go through `decode_sprite` - which shells out to ffmpeg - so opening the Cursor panel would launch 135 subprocesses.
- `files` - kind wire name to filename inside `dir`. Already alias-resolved and already carrying the busy-is-arrow substitution, so the grid needs to know neither rule. A kind the pack does not ship is simply absent, so the grid never points an `<img>` at a file that is not there.
- `busy` - the pack's busy animation, so a hovered tile previews it with the same `busyPose` the export runs. `None` when the busy state is a still.

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

One pack folder's listing entry: `pack::read_meta` plus the folder path, its frame-counted `BusySpec`, and its `files` map. `builtin` comes from the CALLER (the folder it was found in), which is what makes it unspoofable by a `"builtin": true` key in an imported pack's own JSON.

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

## imported_info

```rust
pub fn imported_info(id: String, name: String, dir: &Path) -> CursorPackInfo
```

A freshly imported pack's row, built from what `pack_import::write_pack` just wrote - no `pack.json` read, since the caller already knows the id and name it chose. An import is always v1 (`busy: None`), so its Busy slot maps to its arrow like any other still pack.

### Used by

- `src-tauri/src/export/cursor/pack.rs` - `list_cursor_packs` wraps `list_packs`
- `src-tauri/src/export/cursor/pack_import.rs` - `import_cursor_pack` returns `imported_info`
- `src/editor/panels/CursorPackGrid.tsx` - the whole grid is driven by `CursorPackInfo`
