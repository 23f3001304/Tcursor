# src-tauri/src/settings/bg_asset.rs

The user's own background file: import it INTO the project, describe it, remove it. Backs `BackgroundSettings.asset` (`kind: Image | Video`).

Everything in this file exists to keep a project **portable**. The chosen file is copied into `<project>/background/` and the settings store only `background/<name>` - relative, forward-slashed - so moving or copying the project folder moves the background with it. Nothing outside the project is ever referenced.

*Why no `image` crate:* this build has none (only `png`, for encoding preview frames), and adding a decoder crate to read four extra formats is not a trade worth making when the bundled ffmpeg/ffprobe already reads every accepted format. Probing and thumbnailing both shell out through `export::pipeline::ffio` / `win::sys::proc::ffcmd`.

## BackgroundAssetInfo

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct BackgroundAssetInfo {
    pub rel_path: String, pub kind: String,
    pub width: u32, pub height: u32, pub duration_ms: Option<u64>,
}
```

What the panel's asset card needs: where the file lives (relative), which `BackgroundKind` it is (`"image"` / `"video"`), its pixel size, and - for a video - how long it loops. `duration_ms` is `None` for a still. Mirrored in TS as `BackgroundAssetInfo` (`src/lib/ipc.ts`).

## asset_kind_for

```rust
pub fn asset_kind_for(ext: &str) -> Option<&'static str>
```

`"image"` for png/jpg/jpeg/webp, `"video"` for gif/mp4/webm/mov, `None` for anything else. Case-insensitive.

*Why a GIF is a video:* one decode path serves both, so the export never grows a second animation subsystem. The only place the distinction survives is the editor preview, which cannot play a GIF in a `<video>` element and reaches for `ImageDecoder` instead (`src/editor/stage/gifFrames.ts`) - and it decides that by extension, not by this kind.

## unique_name

```rust
pub fn unique_name(dir: &Path, file_name: &str) -> String
```

`file_name`, or the first of `<stem>_2.<ext>`, `<stem>_3.<ext>`, ... that `dir` does not already hold. A file with no extension dedupes as `<name>_2`.

*Why not overwrite:* an earlier project state may still point at the existing copy. Importing `loop.mp4` twice must produce two files, not one file and one silently changed background.

## resolve_rel

```rust
fn resolve_rel(project_dir: &Path, rel: &str) -> Option<PathBuf>
```

A stored relative path as an absolute one, WITHOUT an existence check. `None` unless the path is relative, non-empty, and made only of plain names - no `..`, no root, no drive prefix. This is the guard that stops a hand-edited `edit.json` from turning the background into an arbitrary-file-read (or, through `remove_background_asset`, an arbitrary-file-delete).

## asset_path

```rust
pub fn asset_path(project_dir: &Path, rel: &str) -> Option<PathBuf>
```

`resolve_rel` plus "and the file is really there". `None` is an everyday case, not an error: a project moved without its `background/` folder, or a file deleted outside the app. Callers fall back to the base wallpaper (`export::scene::background::build`) or show a "File missing" card rather than failing.

## thumb_rel

```rust
pub fn thumb_rel(rel: &str) -> String
```

Where an asset's thumbnail lives, DERIVED from the asset's own relative path so the pair can never drift: `background/clip.mp4` -> `background/.thumbs/clip.mp4.jpg`. The full file name (extension included) is kept, so `a.png` and `a.mp4` get separate thumbs. Mirrored in TS by `panels/backgroundAsset.ts`'s `thumbRel`.

## probe_asset

```rust
pub fn probe_asset(file: &Path, kind: &str) -> (u32, u32, Option<u64>)
```

Pixel size (`ffio::probe_dims`) and, for a video only, duration in ms (`ffio::probe_duration`). A probe that fails reports zeros rather than an error: the import still succeeded and the card simply has less to say.

## write_thumb

```rust
pub fn write_thumb(src: &Path, dest: &Path) -> bool
```

One ffmpeg call writing a 320px-wide JPEG of `src`'s first frame (`-frames:v 1 -vf scale=320:-2 -q:v 3`; `-2` keeps the height even, which the JPEG encoder wants). `false` if ffmpeg is absent or refused the file - a missing thumbnail is cosmetic, never fatal, and the card falls back to a typographic tile.

## import_blocking

```rust
pub fn import_blocking(project_dir: &Path, src: &Path) -> Result<BackgroundAssetInfo, String>
```

The whole import, as a plain function taking its two folders as arguments so it is testable without touching app data: validate the file and its extension, `create_dir_all(<project>/background)`, `unique_name`, copy, thumbnail, probe. Errors name what IS accepted rather than just refusing.

## info_blocking

```rust
pub fn info_blocking(project_dir: &Path, rel: &str) -> Option<BackgroundAssetInfo>
```

Describe an already-imported asset, re-creating a thumbnail that went missing. `None` when the file is gone, which is what makes the card able to say so instead of showing a stale name.

## remove_blocking

```rust
pub fn remove_blocking(project_dir: &Path, rel: &str) -> Result<(), String>
```

Delete the asset and its thumbnail. Idempotent (a second Remove, or an already-deleted file, is success), and bounded by `resolve_rel`, so it can only ever delete inside the project.

## import_background_asset

```rust
#[tauri::command]
pub async fn import_background_asset(project_dir: String, src_path: String) -> Result<BackgroundAssetInfo, String>
```

`import_blocking` on a blocking thread - it copies a file that can be gigabytes and shells out twice, neither of which may run on the UI's async runtime.

## background_asset_info

```rust
#[tauri::command]
pub async fn background_asset_info(project_dir: String, rel_path: String) -> Result<Option<BackgroundAssetInfo>, String>
```

`info_blocking`, same blocking-thread reasoning (an ffprobe per call). Called by the panel on mount, which is how the card stays honest after a project is reopened - at that point `edit.json` carries the relative path and nothing else.

## remove_background_asset

```rust
#[tauri::command]
pub fn remove_background_asset(project_dir: String, rel_path: String) -> Result<(), String>
```

`remove_blocking`. Sync: two `remove_file` calls.

**It does NOT touch `edit.json`.** The doc lives in the frontend and is written by `save_edit`; the panel clears `background.asset` in the same save that follows this call. A settings write from here would be silently overwritten by the next frontend save, and the editor's in-memory doc would disagree with the file in the meantime.

### Behaviors

- `extensions_map_to_the_two_kinds_and_nothing_else` - the eight accepted extensions map as documented (case-insensitive); `exe`/`svg`/`mkv`/`""` map to nothing.
- `a_name_collision_gets_a_numeric_suffix_not_an_overwrite` - `loop.mp4` -> `loop_2.mp4` -> `loop_3.mp4`, and an extension-less name still dedupes.
- `a_relative_path_must_stay_inside_the_project` - `..` traversal, an absolute path, an empty path and a missing file all resolve to `None`.
- `the_thumb_path_is_derived_not_stored` - including a name with a space, and a path with no folder.
- `import_refuses_what_it_cannot_render` - a `.txt` is refused with an error naming the accepted list, a missing file is refused as "not a file", and neither leaves a `background/` folder behind.
- `import_copies_the_file_in_keeps_its_name_and_returns_a_relative_path` (needs ffmpeg; skipped without it) - a real PNG lands at `background/hero.png` with a thumbnail beside it, is probed to its true 8x4, has no duration, dedupes on a second import, and round-trips back through `asset_path`.
- `info_reports_a_missing_asset_as_none_and_remove_is_idempotent` - Remove deletes both files, a second Remove succeeds, `info_blocking` then reports `None`, and a traversal path is refused.

### Used by

- `src-tauri/src/settings/background.rs` (`BackgroundSettings.asset`) - the field this file's paths live in.
- `src-tauri/src/export/scene/background.rs` (`build`, `video_source`) - resolves the asset to real pixels.
- `src-tauri/src/lib.rs` - registers the three commands.
- `src/lib/ipc.ts` (`importBackgroundAsset`, `backgroundAssetInfo`, `removeBackgroundAsset`) - the TS wrappers.
- `src/editor/panels/BackgroundAssetCard.tsx` - the only caller.
