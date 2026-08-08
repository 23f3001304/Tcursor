# src-tauri/src/session/record/mod.rs

Submodule overviews for the `record` group.

## recorder

Top-level recording controller exposed as three Tauri commands. Owns a `Mutex<Option<Running>>` as managed state; each command locks briefly, modifies or consumes the `Running` value, and returns. Key items: `Recorder` (managed-state singleton), `Running` (private struct holding all live resources for one session), `RecordingResult` (returned to the frontend by `stop_recording`), `start_recording` (creates folder, starts all threads), `pause_recording` / `resume_recording` (flip the shared `paused` flag), `stop_recording` (signals all threads, joins in dependency order, writes `sync.json`).

## recorder_threads

Thread-spawning helpers and input-persistence logic factored out of `recorder.rs` to keep that file under the 200-line cap. Key items: `save_inputs` (stops each input tracker and writes `events.json`, `actions.json`, `typing.json`, `cursor.json` before the video thread is joined), `spawn_mic_thread` (spawns the `CpalMic` holder thread or returns `None` if mic is off), `spawn_system_thread` (spawns the `SystemAudio` loopback holder thread or returns `None` if disabled).

## recording_session

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Runs entirely on the `"video"` thread; has no threading primitives of its own. Key items: `RecordingSession` (owns source and sink, tracks frame count and timestamps), `RecordingSession::new`, `RecordingSession::pump_once` (pulls one frame and pushes to encoder), `RecordingSession::run` (variable-FPS loop with pause support), `RecordingSession::run_paced` (delegates to `pacing::run_paced` for CFR mode), `RecordingSession::stop_and_finalize` (flushes the sink and returns the frame count), `SessionState` (lifecycle enum).

## pause_clock

Private helper module (not re-exported beyond `record`) shared by `gpu_record::Cap` and `recording_session::RecordingSession`. Key item: `PauseClock` (accumulates paused wall-clock time from a stream of per-frame `(now, paused)` samples and shifts capture timestamps to exclude it, so a pause leaves no gap between `sync.json` and `video.mp4`).
