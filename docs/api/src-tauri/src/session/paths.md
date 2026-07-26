# src-tauri/src/session/paths.rs

Single source of truth for all per-project file paths. Every track, metadata file, and sync artifact lives under one named folder; callers derive concrete paths by calling methods on `ProjectPaths` rather than constructing strings by hand. Pure value type - no I/O except in `ensure`, no synchronization needed.

## ProjectPaths

```rust
pub struct ProjectPaths { pub folder: PathBuf }
```

- `folder: PathBuf` - the root directory for one recording project, e.g. `Videos/TCursor/my-recording`. *Why a single field:* all artifact paths are `folder.join(filename)`, so the struct is the folder - methods derive the rest.

### Used by

- `src-tauri/src/session/record/recorder.rs` - constructs `ProjectPaths` from `base + project_name`, calls `ensure`, then calls every path accessor to wire up the recording.
- `src-tauri/src/export/pipeline/exporter.rs` - loads paths to locate `video.mp4`, `events.json`, `sync.json`, etc. during export.
- `src-tauri/src/export/pipeline/audio_mux.rs` - reads `mic.wav` and `system.wav` paths to mux audio tracks.
- `src-tauri/src/export/pipeline/timeline.rs` - reads `sync.json` path to load `SyncLog`.
- `src-tauri/src/export/pipeline/run.rs` - obtains `ProjectPaths` to hand off to `exporter.rs`.
- `src-tauri/src/edit/commands.rs` - reads `edit.json` and `settings.json` paths for the editor.
- `src-tauri/src/edit/seed.rs` - reads `events.json`, `typing.json`, `actions.json`, and `sync.json` to seed the edit plan.
- `src-tauri/src/ai/commands.rs` - reads `edit.json` path for AI-director commands.
- `src-tauri/src/session/project/commands.rs` (`folder_from_manifest_path`) - builds a `ProjectPaths` from a resolved folder to call `.manifest()`.

## ProjectPaths::new

```rust
pub fn new(base: &Path, name: &str) -> Self
```

Constructs a `ProjectPaths` rooted at `base.join(name)`.

### Inputs

- `base: &Path` - parent directory, typically `Videos/TCursor`. *Why not hardcoded:* the caller (`recorder.rs`) resolves the OS video directory via `dirs_next::video_dir`, falling back to `temp_dir`, keeping path resolution in one place.
- `name: &str` - project identifier, e.g. a timestamp string. *Why a plain `&str`:* callers own the string; no allocation needed until the `join`.

### Returns

A `ProjectPaths` with `folder = base.join(name)`. The folder is not created here; call `ensure` before writing.

## ProjectPaths::video

```rust
pub fn video(&self) -> PathBuf
```

Returns `folder/video.mp4` - the H.264 screen capture output.

## ProjectPaths::mic

```rust
pub fn mic(&self) -> PathBuf
```

Returns `folder/mic.wav` - the microphone audio track.

## ProjectPaths::events

```rust
pub fn events(&self) -> PathBuf
```

Returns `folder/events.json` - the `EventLog` (mouse events + screen info).

## ProjectPaths::system

```rust
pub fn system(&self) -> PathBuf
```

Returns `folder/system.wav` - the system audio loopback track.

## ProjectPaths::webcam

```rust
pub fn webcam(&self) -> PathBuf
```

Returns `folder/webcam.webm` - the webcam video blob written by the frontend via `save_webcam`.

## ProjectPaths::sync

```rust
pub fn sync(&self) -> PathBuf
```

Returns `folder/sync.json` - the `SyncLog` mapping each frame to its real capture timestamp and recording audio start offsets.

## ProjectPaths::settings

```rust
pub fn settings(&self) -> PathBuf
```

Returns `folder/settings.json` - a snapshot of the active `Settings` at recording start, so the export always reproduces the zoom/cursor config used during capture.

## ProjectPaths::actions

```rust
pub fn actions(&self) -> PathBuf
```

Returns `folder/actions.json` - the `ActionLog` of keyboard action events.

## ProjectPaths::typing

```rust
pub fn typing(&self) -> PathBuf
```

Returns `folder/typing.json` - the `TypingLog` of keystroke timestamps used by the smart-zoom heuristic.

## ProjectPaths::cursor

```rust
pub fn cursor(&self) -> PathBuf
```

Returns `folder/cursor.json` - the `CursorTrack` of cursor-type samples used by the enhanced cursor renderer.

## ProjectPaths::edit

```rust
pub fn edit(&self) -> PathBuf
```

Returns `folder/edit.json` - the editor's `EditPlan` produced by the AI director or manual edits.

## ProjectPaths::manifest

```rust
pub fn manifest(&self) -> PathBuf
```

Returns `folder/project.tcursor` - the `ProjectManifest` written at record-stop (`stop_recording`) and read by `open_project` / the file-association cold-start path (`folder_from_manifest_path`). The one place that filename is spelled, so the writer and every reader can never disagree on it.

### Behaviors

- `builds_manifest_path`: confirms `manifest()` ends with `project.tcursor`.

## ProjectPaths::ensure

```rust
pub fn ensure(&self) -> std::io::Result<()>
```

Creates `folder` (and any missing parents) via `std::fs::create_dir_all`.

### Inputs

- `&self` - only the `folder` field is used. *Why not a static method:* callers always have a `ProjectPaths` in hand before writing files, so `ensure` fits naturally.

### Returns

`Ok(())` on success. Propagates the first `io::Error` from `create_dir_all` (e.g. permission denied). The caller in `recorder.rs` maps the error to a `String` and returns it to the frontend.

### Behaviors

- `builds_track_paths_under_named_folder`: confirms `folder` ends with `rec1`, `video()` ends with `video.mp4`, `mic()` with `mic.wav`, `events()` with `events.json`.
- `builds_system_and_webcam_paths`: confirms `system()` ends with `system.wav`, `webcam()` ends with `webcam.webm`.
- `builds_settings_snapshot_path`: confirms `settings()` ends with `settings.json`.
- `builds_actions_path`: confirms `actions()` ends with `actions.json`.
- `builds_typing_path`: confirms `typing()` ends with `typing.json`.
- `builds_cursor_path`: confirms `cursor()` ends with `cursor.json`.
- `builds_edit_path`: confirms `edit()` ends with `edit.json`.
