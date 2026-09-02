# src-tauri/src/session/record/mod.rs

Submodule overviews for the `record` group, plus the two shared items the whole group is wired with.

## Notify

```rust
pub type Notify = Arc<dyn Fn(&str) + Send + Sync>;
```

A one-way notification from a recording thread to the app: the reason string is what the HUD shows the user. *Why an `Arc<dyn Fn>` and not an `AppHandle`:* the capture/encode pipeline then carries no Tauri types, stays constructible in tests, and the one place that knows about events is `recorder::emitter`. Two are built per recording - `record-warning` (a degraded but still-running take, e.g. an audio input that would not open) and `record-ended-early` (the OS ended the capture, or the capture's own dimensions changed mid-record - `CAPTURE_CLOSED` and `DISPLAY_CHANGED` are the two reason strings sent through the latter).

## CAPTURE_CLOSED

```rust
pub const CAPTURE_CLOSED: &str = "The recorded window or display closed. The recording was saved up to that point.";
```

The reason passed to the capture-ended `Notify` when the OS - not the user - ends the capture: the recorded window was closed, or the recorded display was unplugged/disabled/slept. Both capture paths report it with the same wording (`gpu_frames::Cap::on_closed` and the ffmpeg video thread in `video_sink::start_ffmpeg`), so the HUD has exactly one message to show.

## DISPLAY_CHANGED

```rust
pub const DISPLAY_CHANGED: &str = "Display changed — recording saved up to the change.";
```

The reason passed to the same `Notify` when the capture's OWN dimensions change mid-record (finding H1) - a recorded window maximized/restored/snapped, or a recorded display changed resolution, rotated, or was docked/undocked. Neither capture path can keep encoding once that happens (the encoder/pipe is sized once, at start): the legacy path used to silently discard every frame from that instant on (`FfmpegFrameSink::write_or_skip`'s `Ok(false)` skip, forever), and the GPU path never checked at all. Both now end the take through this same early-end signal on the FIRST mismatched frame - `gpu_frames::Cap::on_frame_arrived` via `dim_guard::DimGuard`, and `recording_session::RecordingSession::pump_once` via its `mismatched` flag, read by `video_sink::start_ffmpeg` - and this distinct wording is what tells the HUD (and the user) it was a size change, not a closed window or display.

## recorder

Recording state plus the Start/Pause/Resume commands. Owns a `Mutex<Option<Running>>` as managed state; each command locks briefly, modifies or consumes the `Running` value, and returns. Key items: `Recorder` (managed-state singleton, with the `stopping` flag that keeps a start out of a stop's teardown window), `Running` (holds all live resources for one session), `emitter` (builds a `Notify` over an `AppHandle`), `start_recording` (creates folder, starts all threads), `pause_recording` / `resume_recording` (stamp the `PauseTotals` ledger and flip the shared `paused` flag).

## recorder_stop

The Stop half of the command surface. Key items: `stop_recording` (the `async` + `spawn_blocking` command), `stop_blocking` (signals all threads, joins in dependency order, saves inputs, finalizes the video, then writes `sync.json` / `project.tcursor` / recents from whatever was recorded - even when the finalize failed), `RecordingResult` (returned to the frontend).

## close_guard

The `CloseRequested` safety net (task-6, ruling R6): if the main window tries to close while `Recorder::is_busy()`, `lib.rs`'s `on_window_event` guard prevents it and hands off here. Key items: `finish_and_close` (finalizes the take directly via `recorder_stop::stop_recording` if nobody has claimed the stop yet, otherwise waits for whoever did, then closes the window - deliberately independent of the frontend, so the OS close button / Alt+F4 / a wedged renderer can never lose a take).

## recorder_threads

Thread-spawning helpers and persistence logic factored out of `recorder.rs` to keep that file under the 200-line cap. Key items: `save_inputs` (stops each input tracker and writes `events.json`, `actions.json`, `typing.json`, `cursor.json` before the video thread is joined), `save_session_files` (`sync.json` + `project.tcursor` + recents), `spawn_mic_thread` / `spawn_system_thread` (spawn the audio holder threads, or return `None` when that input is off), `audio_warning` (the `record-warning` message for an input that would not open).

## recording_session

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Runs entirely on the `"video"` thread; has no threading primitives of its own. Key items: `RecordingSession` (owns source and sink, tracks frame count, timestamps, and whether a dimension mismatch ended the take), `RecordingSession::new`, `RecordingSession::pump_once` (pulls one frame and pushes it to the encoder at its pause-compressed timestamp; stops - and latches `dimension_mismatch()` - on the FIRST dimension-mismatched frame instead of skipping it forever), `RecordingSession::run` (variable-FPS loop with pause support, splitting the sink on each resume), `RecordingSession::run_paced` (delegates to `pacing::run_paced` for CFR mode), `RecordingSession::stop_and_finalize` (flushes the sink and returns the frame count), `RecordingSession::dimension_mismatch`, `SessionState` (lifecycle enum).

## dim_guard

Pure decision point for the GPU path's half of the same dimension-change detection, split out so it is testable without live WGC or a real `VideoEncoder`. Key items: `DimGuard` (latches the first frame whose size no longer matches the encoder's configured `(w, h)`).

## gpu_record

The default capture path: WGC surfaces straight into the Media Foundation `VideoEncoder`, no readback. Key items: `GpuRecorder` (lifecycle + shared per-frame timestamps), `GpuStart` (its start config), `target_bitrate` / `video_settings` / `encoder` (encoder configuration).

## gpu_frames

The GPU path's frame callback, split out of `gpu_record.rs`. Key items: `Cap` (the `GraphicsCaptureApiHandler` - rebases each frame's encoder PTS onto the recording clock, ends the take on a mid-record dimension change via `DimGuard`, and reports an OS-closed capture), `CapFlags`, `FrameTimes`.

## video_sink

Picks between the two capture paths and normalises their stop. Key items: `VideoSink` (the enum), `VideoStart` (shared start config), `VideoStopped` (frames + timestamps + an optional finalize error, so a failure never costs the session files), `start_video`, `VideoSink::stop_and_collect`.

## pause_clock

Private helper module (not re-exported beyond `record`) shared by `gpu_frames::Cap` and `recording_session::RecordingSession`. Key items: `PauseClock` (places each arriving frame on the recording clock from the shared `PauseTotals` ledger), `FrameTick` (that placement: the `sync.json` timestamp and the matching encoder PTS, from one ledger read).

## pause_totals

The exact-span paused-time ledger, stamped at the pause/resume instants under the recorder lock and shared by every input tracker AND both capture paths. Key items: `PauseTotals`, `PauseTotals::pause` / `resume` / `elapsed_paused`, `PauseTotals::stamp_ms` (the one implementation of "remove the paused time"), `PauseTotals::stamp` (its `u32` form for input trackers).
