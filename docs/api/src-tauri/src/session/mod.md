# src-tauri/src/session/mod.rs

MODULE OVERVIEW: The `session` module orchestrates everything that happens between "start recording" and "stop recording". It owns the recording lifecycle (Tauri commands, thread management, managed state) and provides the supporting infrastructure those threads need: frame pacing, file paths, the frame-push loop, and the timing log that ties all tracks together. `recorder` is the top-level entry point, exposed as Tauri commands; it delegates thread spawning to `recorder_threads`, the frame-pump state machine to `recording_session`, constant-frame-rate logic to `pacing`, artifact paths to `paths`, and post-recording timing metadata to `sync`. Data flows outward: inputs arrive on dedicated threads, frames are encoded by `ffmpeg_encoder`, and at stop time all artifacts are written to the folder identified by `paths` and summarized in `sync.json`.

## pacing

Constant-frame-rate (CFR) capture helpers for game-mode recording. Decoupled from `RecordingSession` so the pacing algorithm is independently testable. Key items: `frames_due` (computes how many frame indices should exist at a given unpaused elapsed time), `emit_due` (emits all not-yet-sent frames due by the current active time, duplicating the last real frame as needed), `run_paced` (main CFR capture loop spinning at 2ms resolution with pause and stop support).

## paths

Single source of truth for all per-project file paths. All callers derive artifact locations from `ProjectPaths` methods rather than building strings by hand, so any rename is a one-place change. Key items: `ProjectPaths` (single-field struct wrapping the project folder), `ProjectPaths::new` (constructs from base dir and project name), `ProjectPaths::ensure` (creates the folder tree), and accessors `video`, `mic`, `system`, `events`, `sync`, `settings`, `actions`, `typing`, `cursor`, `edit`.

## sync

Per-recording timing log that anchors every captured track to the shared capture clock. Written once at `stop_recording` and read by the exporter to reconstruct the real frame timeline and compute audio offsets. Key items: `SyncLog` (fields: `frames` per-frame timestamps, `events_ms` mouse-clock origin, `mic_ms` and `system_ms` audio start times), `SyncLog::save`, `SyncLog::load`.

## project

The `.tcursor` project format: `project.tcursor`, a small JSON manifest written into the project folder (never a zip/copy of the multi-GB video), plus the "recently opened" list and the `open_project`/file-association command surface. Key items: `manifest::ProjectManifest` (`save`/`load`/`new`/`load_or_default`), `recents::RecentProject` (`list`/`touch`), `commands::open_project`/`list_recent_projects`/`get_launch_project`, `commands::LaunchProject` (cold-start managed state), `commands::folder_from_manifest_path` / `launch_project_from_argv`.
