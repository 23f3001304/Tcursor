# src-tauri/src/export/cursor/pack/packdirs.rs

Where cursor packs live on disk. There are three sources, and this file is the only place that knows the difference between them:

| source | location | `builtin` | deletable |
| --- | --- | --- | --- |
| embedded ("default") | bytes compiled into the binary (`cursorset::SPRITES`); the same PNGs also ship at `assets/cursors/` so the grid has something to show | yes | no |
| **bundled** | `assets/cursorpacks/<id>/` in the app's resources | yes | no |
| imported | `<config-dir>/TCursor/cursors/<id>/` | no | yes |

Bundled and imported packs are the *same folder shape*, so everything downstream (`pack::sprite_sources`, `pack::busy_spec`, the preview's `cursor_sprites`) takes a plain `&Path` and never asks which kind it got. That is what makes `CursorSettings.pack = "cat"` just work in the export and the preview with no special case.

## cursors_dir

```rust
pub fn cursors_dir() -> PathBuf
```

`<config-dir>/TCursor/cursors` - one subfolder per IMPORTED pack. Sibling to `settings::store::config_path`'s `TCursor/config.json` (same app-data convention: `dirs_next::config_dir()`, falling back to a temp dir).

## pack_dir

```rust
pub fn pack_dir(pack_id: &str) -> PathBuf
```

Where an imported pack's PNGs + `hotspots.json` + `pack.json` live: `cursors_dir()/<pack_id>`. Returned whether or not the folder exists - callers test for themselves.

## resource_candidates

```rust
pub fn resource_candidates(rel: &str, resource_dir: Option<&Path>) -> Vec<PathBuf>
```

Roots that may hold `rel` (an `assets/...` resource folder), most-specific first - the same shape, and the same reasoning, as `process::proc::ffmpeg_candidates`.

1. **Next to the running executable.** The installed build: `bundle.resources` lists `assets/cursorpacks` (see `tauri.conf.json`), which Tauri copies under the install dir preserving that relative path.
2. **One and two levels above it.** `cargo run`, `cargo test` and `tauri dev` all run the binary out of `src-tauri/target/<profile>/` (or `.../deps/` for tests), so walking up reaches `src-tauri/`, and `src-tauri/assets/...` IS the repo's own asset folder. **This is why dev needs no staging step and no copy** - the same files the bundle ships are already there, and `packdirs_tests::the_shipped_packs_resolve_from_the_running_test_binary` proves it from inside the test binary, which sits one level deeper than the dev exe.
3. **Tauri's own resource dir**, when a caller has an `AppHandle` to ask (only `pack::list_cursor_packs` does). Belt and braces: the entries above already resolve in both dev and an installed build.

## bundled_root

```rust
pub fn bundled_root(resource_dir: Option<&Path>) -> Option<&'static Path>
```

The first candidate root that exists, memoized in a `OnceLock` for the life of the process.

*Why memoizing on the first caller is safe even though some callers pass `None`:* the exe-relative candidates come FIRST and are the ones that actually resolve in both dev and an installed build, so a later call carrying a resource dir cannot disagree with an earlier one that did not. That is what lets `sprite_sources` - reached from `FrameRenderer::new`, which has no `AppHandle` - resolve a bundled pack at all.

## default_pack_dir

```rust
pub fn default_pack_dir(resource_dir: Option<&Path>) -> Option<&'static Path>
```

The EMBEDDED pack's folder (`assets/cursors`, a sibling of `assets/cursorpacks`), memoized in its own `OnceLock` the same way.

*Why the embedded pack needs a folder at all:* its bytes are compiled into the binary, which is what the export reads and will keep reading - but the panel's grid loads tiles through the asset protocol and cannot show `include_bytes!`. Shipping the same PNGs as a resource is what lets "Default" have a tile that shows a sprite and cycles on hover like every other pack. `None` when the resource is missing: the pack still lists and still exports, its tile just shows the name alone.

Its filenames predate the pack format (`pointer.png` for the arrow); `packlist::default_filename` is the one-entry alias map.

## bundled_pack_dir

```rust
pub fn bundled_pack_dir(id: &str) -> Option<PathBuf>
```

The bundled folder for `id`, if there is one. Two callers: resolving a selected pack, and keeping `pack_import::unique_id` from ever handing an import a bundled pack's id.

## bundled_pack_dirs

```rust
pub fn bundled_pack_dirs(resource_dir: Option<&Path>) -> Vec<PathBuf>
```

Every bundled pack folder, alphabetical by folder name so the grid's order is stable across runs.

## imported_pack_dirs

```rust
pub fn imported_pack_dirs() -> Vec<PathBuf>
```

Every imported pack folder under `cursors_dir()`, alphabetical. Empty for a missing or unreadable folder (the normal state until the user imports their first pack).

## dirs_in

```rust
fn dirs_in(root: &Path) -> Vec<PathBuf>
```

Sorted subdirectories of `root`; empty when it cannot be read. The shared core of both listings, and what the tests exercise against an explicit temp root (the memoized `bundled_root` has one slot per process, so tests must not race each other for it).

## resolve_pack_dir

```rust
pub fn resolve_pack_dir(pack_id: &str) -> PathBuf
```

The folder a selected pack id resolves to: a **bundled** pack first, then an imported one.

*Why bundled wins:* `pack::list_packs` lists it that way too and drops any later pack with an id already taken, so one id is one pack in the grid AND in the renderer - the preview and the export can never pick different folders for the same selection. A pack imported *before* a bundled one claimed that id is the only casualty; `pack_import::unique_id` makes sure no NEW import can land in that position.

### Used by

- `src-tauri/src/export/cursor/pack.rs` - `sprite_sources`, `busy_spec`, `busy_frames`, `list_packs`
- `src-tauri/src/export/cursor/pack/pack_import.rs` - `unique_id`'s collision guard, and the destination for a new import
