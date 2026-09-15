# src-tauri/src/export/cursor/pack/pack_import.rs

Import a user-chosen folder (the pack format documented in `pack.rs`: `arrow.png`/`ibeam.png`/`hand.png`/.../`hotspots.json`) as a new cursor pack. Validates the folder has at least one recognized sprite, copies the recognized files under `pack::cursors_dir()`, and hands back a `CursorPackInfo` the frontend can select immediately.

## import_cursor_pack

```rust
#[tauri::command]
pub fn import_cursor_pack(path: String) -> Result<CursorPackInfo, String>
```

Validate `path` is a folder containing at least one recognized cursor PNG, then copy the recognized PNGs + `hotspots.json` (if present and valid) into a new `cursors_dir()` subfolder.

### Inputs

- `path: String` - absolute path to a folder the user picked (via the frontend's folder dialog). *Why a folder, not a zip:* matches the pack format as specified (a plain folder of PNGs); keeps this import path free of an archive-extraction dependency.

### Returns

`Ok(CursorPackInfo)` with `builtin: false` on success. `Err(String)` when: `path` isn't a directory; the folder has no recognized cursor PNGs (a PNG whose filename matches `pack::kind_filename` for some `CursorType` and decodes with `ffio::png_dims`); the folder's `hotspots.json` exists but isn't valid `{kind: [hx, hy]}` JSON; or the copy/write to disk fails.

### Implementation

1. Reject a non-directory `path`.
2. Derive `name` from the source folder's own name (falls back to `"Cursor Pack"` if unrepresentable as UTF-8).
3. `collect_valid_sprites` - if empty, error (no recognized PNGs).
4. `read_and_validate_hotspots` - propagates a parse error; `None` if the file is simply absent.
5. `id = unique_id(&slugify(&name))` - a filesystem-safe, collision-free id.
6. `write_pack(&pack_dir(&id), &id, &name, &sprites, hotspots_json)` - on failure, best-effort `remove_dir_all` the partial pack dir before returning the error, so a retry doesn't see a half-imported pack.
7. Return the new `CursorPackInfo`.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`
- `src/shared/ipc.ts` (`importCursorPack`) - the TS wrapper
- `src/editor/panels/cursor/CursorPanel.tsx` - calls after the user picks a folder via the Tauri dialog plugin, then selects the returned pack

## collect_valid_sprites

```rust
fn collect_valid_sprites(src: &Path) -> Vec<(String, Vec<u8>)>
```

Every `(filename, bytes)` in `src` that both names a recognized cursor kind and decodes as a PNG with real (nonzero) dimensions. Kinds the folder doesn't provide (or provides invalid files for) are silently skipped - the pack still imports; `pack::sprite_sources` fills the gap with the built-in sprite for those kinds at render time.

### Implementation

For each `(kind, ..)` in `cursorset::SPRITES`: read `src/{kind_filename(kind)}`; validate with `ffio::png_dims` (real PNG signature + nonzero width/height); keep `(filename, bytes)` on success, skip on any failure (missing file, unreadable, not a valid PNG).

### Behaviors

- `collect_valid_sprites_only_keeps_recognized_names_with_real_png_bytes` - a folder with a valid `arrow.png`, a `hand.png` containing non-PNG bytes, and an unrecognized `unknown_name.png` (valid PNG bytes, wrong name) yields only the one valid, recognized entry.

## read_and_validate_hotspots

```rust
fn read_and_validate_hotspots(src: &Path) -> Result<Option<Vec<u8>>, String>
```

`hotspots.json` in `src`, if present: `Ok(Some(raw bytes))` when it parses as a `{kind_name: [hx, hy]}` map, `Ok(None)` when the file is simply absent (every hotspot then defaults to a centered `(0.5, 0.5)` per `pack::sprite_sources`), `Err` when present but not valid JSON in that shape - a real authoring mistake worth surfacing rather than silently discarding.

### Behaviors

- `read_and_validate_hotspots_ok_none_when_file_absent`
- `read_and_validate_hotspots_ok_some_when_shape_matches`
- `read_and_validate_hotspots_errs_on_malformed_json`

## slugify

```rust
fn slugify(name: &str) -> String
```

Lowercase alnum-only slug for a pack id seed (`"My Pack!"` -> `"my_pack"`), never empty (falls back to `"pack"` if the input has no alphanumeric characters at all, e.g. an all-punctuation or blank folder name).

### Behaviors

- `slugify_lowercases_and_replaces_punctuation` - `"My Cool Pack!"` -> `"my_cool_pack"`; `"  "` and `""` both -> `"pack"`; an already-clean name passes through unchanged (just lowercased).

## unique_id

```rust
fn unique_id(base: &str) -> String
```

`base`, or the first of `base_2`, `base_3`, ... that no pack already answers to.

**BOTH sources are checked**: an imported folder under `cursors_dir()`, and a BUNDLED pack of that id. Bundled packs win at resolution time (`packdirs::resolve_pack_dir`), so handing an import a bundled id would silently make the import unreachable - the user would pick their own pack and get the shipped one. It also never returns `"default"`, since the embedded set has no folder and `slugify` cannot produce a collision the loop would miss.

## write_pack

```rust
fn write_pack(dir: &Path, id: &str, name: &str, sprites: &[(String, Vec<u8>)], hotspots_json: Option<&[u8]>) -> std::io::Result<()>
```

Write the new pack folder at `dir`: each recognized sprite PNG, `hotspots.json` (the source file verbatim, or `{}` if it had none), and `pack.json` (`{id, name}`, read back by `pack::list_cursor_packs`'s `read_pack_meta`).

### Inputs

- `dir: &Path` - the destination folder. Takes it directly (rather than deriving it from `id` via `pack::pack_dir` internally) so this function is unit-testable against a temp folder, independent of the real app-data `cursors_dir()`.
- `id: &str`, `name: &str` - written verbatim into `pack.json`.
- `sprites: &[(String, Vec<u8>)]` - `(filename, PNG bytes)` pairs from `collect_valid_sprites`.
- `hotspots_json: Option<&[u8]>` - the validated raw bytes from `read_and_validate_hotspots`, or `None`.

### Returns

`std::io::Result<()>` - propagates the first filesystem error (directory creation or any file write).

### Behaviors

- `write_pack_creates_pngs_hotspots_and_meta` - creates a not-yet-existing nested folder, writes the sprite PNG, and both metadata files with the given hotspot data.
- `write_pack_defaults_hotspots_to_empty_object_when_none_given` - `hotspots_json: None` writes literal `"{}"`.
