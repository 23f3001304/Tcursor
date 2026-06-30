# src-tauri/src/session/mod.rs

MODULE OVERVIEW: The `session` module orchestrates everything that happens between "start recording" and "stop recording". It owns the recording lifecycle (Tauri commands, thread management, managed state) and provides the supporting infrastructure those threads need: frame pacing, file paths, the frame-push loop, and the timing log that ties all tracks together. `recorder` is the top-level entry point, exposed as Tauri commands; it delegates thread spawning to `recorder_threads`, the frame-pump state machine to `recording_session`, constant-frame-rate logic to `pacing`, artifact paths to `paths`, and post-recording timing metadata to `sync`. Data flows outward: inputs arrive on dedicated threads, frames are encoded by `ffmpeg_encoder`, and at stop time all artifacts are written to the folder identified by `paths` and summarized in `sync.json`.

## pacing

Constant-frame-rate (CFR) capture helpers for game-mode recording. Decoupled from `RecordingSession` so the pacing algorithm is independently testable. Key items: `frames_due` (computes how many frame indices should exist at a given unpaused elapsed time), `emit_due` (emits all not-yet-sent frames due by the current active time, duplicating the last real frame as needed), `run_paced` (main CFR capture loop spinning at 2ms resolution with pause and stop support).

## paths

Single source of truth for all per-project file paths. All callers derive artifact locations from `ProjectPaths` methods rather than building strings by hand, so any rename is a one-place change. Key items: `ProjectPaths` (single-field struct wrapping the project folder), `ProjectPaths::new` (constructs from base dir and project name), `ProjectPaths::ensure` (creates the folder tree), and accessors `video`, `mic`, `system`, `events`, `sync`, `settings`, `actions`, `typing`, `cursor`, `edit`.

## recorder

Top-level recording controller exposed as three Tauri commands. Owns a `Mutex<Option<Running>>` as managed state; each command locks briefly, modifies or consumes the `Running` value, and returns. Key items: `Recorder` (managed-state singleton), `Running` (private struct holding all live resources for one session), `RecordingResult` (returned to the frontend by `stop_recording`), `start_recording` (creates folder, starts all threads), `pause_recording` / `resume_recording` (flip the shared `paused` flag), `stop_recording` (signals all threads, joins in dependency order, writes `sync.json`).

## recorder_threads

Thread-spawning helpers and input-persistence logic factored out of `recorder.rs` to keep that file under the 200-line cap. Key items: `save_inputs` (stops each input tracker and writes `events.json`, `actions.json`, `typing.json`, `cursor.json` before the video thread is joined), `spawn_mic_thread` (spawns the `CpalMic` holder thread or returns `None` if mic is off), `spawn_system_thread` (spawns the `SystemAudio` loopback holder thread or returns `None` if disabled).

## recording_session

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Runs entirely on the `"video"` thread; has no threading primitives of its own. Key items: `RecordingSession` (owns source and sink, tracks frame count and timestamps), `RecordingSession::new`, `RecordingSession::pump_once` (pulls one frame and pushes to encoder), `RecordingSession::run` (variable-FPS loop with pause support), `RecordingSession::run_paced` (delegates to `pacing::run_paced` for CFR mode), `RecordingSession::stop_and_finalize` (flushes the sink and returns the frame count), `SessionState` (lifecycle enum).

## sync

Per-recording timing log that anchors every captured track to the shared capture clock. Written once at `stop_recording` and read by the exporter to reconstruct the real frame timeline and compute audio offsets. Key items: `SyncLog` (fields: `frames` per-frame timestamps, `events_ms` mouse-clock origin, `mic_ms` and `system_ms` audio start times), `SyncLog::save`, `SyncLog::load`.
