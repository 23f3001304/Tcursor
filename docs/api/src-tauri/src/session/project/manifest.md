# src-tauri/src/session/project/manifest.rs

`project.tcursor`: a small JSON manifest written into the project folder - never a zip/copy of the multi-GB video (the folder itself IS the project). Written once, `preprocessed: false`, at `stop_recording`; `export::preview::preprocess::preprocess_project` flips `preprocessed` to `true` after generating proxies/thumbs/waveform/preview-audio/edit.json ahead of time (run from the HUD's "Saving..." step, right after `stop_recording` resolves) so the editor can skip regenerating them. `load_or_default` is the back-compat seam: recordings made before this feature (or a project whose preprocessing pass failed partway) have no manifest, or one with `preprocessed: false`, and every caller (`open_project`, the file-association cold-start path, `get_project_manifest`) must still be able to open them - falling back to the editor's own lazy `ensure_*` generation.

## MANIFEST_VERSION

```rust
pub const MANIFEST_VERSION: u32 = 1;
```

Bumped only if the manifest's shape changes in a way readers must branch on (e.g. a field is removed or repurposed). Adding a new field with `#[serde(default)]` does not require a bump - existing manifests just deserialize with the new field defaulted.

## ProjectManifest

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectManifest {
    pub version: u32,
    pub created_unix_ms: u64,
    pub source_w: u32,
    pub source_h: u32,
    pub app_version: String,
    #[serde(default)]
    pub preprocessed: bool,
}
```

- `version: u32` - `MANIFEST_VERSION` at write time.
- `created_unix_ms: u64` - wall-clock time the recording finished, matching the `started_unix_ms` convention used elsewhere in `session` (e.g. `EventLog`).
- `source_w: u32` / `source_h: u32` - the captured screen resolution (`ScreenInfo.w`/`.h` at `stop_recording`). `0`/`0` signals "unknown source" from `load_or_default`'s synthesized fallback, never a real captured size.
- `app_version: String` - `env!("CARGO_PKG_VERSION")` of the build that wrote the manifest. *Why the crate version, not `tauri::AppHandle::package_info`:* avoids threading an `AppHandle` into `stop_recording` just for this; the crate version and `tauri.conf.json`'s `version` are kept in step already.
- `preprocessed: bool` - `#[serde(default)]` so a manifest written by an older build (before this field existed) still deserializes, defaulting to `false` (safe: "not yet known to be preprocessed" is the conservative reading).

### Used by

- `src-tauri/src/session/record/recorder.rs` (`stop_recording`) - constructs and saves one per finished recording (`preprocessed: false`).
- `src-tauri/src/export/preview/preprocess.rs` (`preprocess_project`) - loads it, flips `preprocessed = true`, and re-saves it after a full successful preprocessing pass.
- `src-tauri/src/session/project/commands.rs` (`folder_from_manifest_path`) - reads it via `load_or_default` when resolving `open_project`/cold-start; (`get_project_manifest`) - reads it via `load_or_default` for the frontend's "already preprocessed?" check.
- `src/lib/ipc.ts` (`ProjectManifest`, `getProjectManifest`) - the TS mirror + getter, consumed by `useEditorData` to skip lazy `ensure_*` calls.

## ProjectManifest::new

```rust
pub fn new(source_w: u32, source_h: u32) -> Self
```

A fresh manifest for a just-finished recording.

### Inputs

- `source_w: u32`, `source_h: u32` - the captured screen resolution.

### Returns

`ProjectManifest` with `version: MANIFEST_VERSION`, `created_unix_ms` = current wall-clock time, `app_version` = this build's crate version, and `preprocessed: false`.

## ProjectManifest::save

```rust
pub fn save(&self, path: &Path) -> io::Result<()>
```

Serializes to pretty-printed JSON and writes it to `path`. Mirrors `SyncLog::save` / `EventLog::save`'s signature convention (caller supplies the full path, typically `ProjectPaths::manifest()`).

### Returns

`Ok(())` on success. Propagates the first `io::Error` from serialization or the write.

## ProjectManifest::load

```rust
pub fn load(path: &Path) -> io::Result<ProjectManifest>
```

Strict load: reads `path` and parses it as a `ProjectManifest`.

### Returns

`Err` if the file doesn't exist, can't be read, or isn't valid JSON in this shape. Callers that need to tolerate a missing/corrupt manifest use `load_or_default` instead.

### Behaviors

- `round_trips_through_json` - `save` then `load` reproduces every field, including a `created_unix_ms` greater than zero.
- `load_errs_when_the_file_is_missing`
- `load_errs_when_the_file_is_not_valid_json`

## ProjectManifest::load_or_default

```rust
pub fn load_or_default(path: &Path) -> ProjectManifest
```

`Self::load(path)`, or a fresh "unknown source" default if the manifest is missing, unreadable, or corrupt.

### Inputs

- `path: &Path` - typically `ProjectPaths::manifest()`. *Why a path, not a folder:* mirrors `load`/`save`'s convention, and lets `folder_from_manifest_path` pass the exact path it already resolved.

### Returns

The real manifest when `load` succeeds. Otherwise a synthesized `ProjectManifest { version: MANIFEST_VERSION, created_unix_ms: 0, source_w: 0, source_h: 0, app_version: env!("CARGO_PKG_VERSION"), preprocessed: false }` - `source_w`/`source_h` of `0` is a deliberate "unknown" signal rather than a fabricated real resolution.

### Behaviors

- `load_or_default_falls_back_to_an_unknown_source_manifest_when_missing` - a folder with no `project.tcursor` at all (an existing pre-feature recording) still resolves, with `source_w`/`source_h` of `0`.
- `load_or_default_falls_back_when_the_file_is_corrupt_too`
- `load_or_default_returns_the_real_manifest_when_present`

### Used by

- `src-tauri/src/session/project/commands.rs` (`folder_from_manifest_path`) - the sole caller today.
