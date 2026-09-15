# src-tauri/src/session/record/mod.rs

Submodule overviews for the `record` group, plus the two shared items the whole group is wired with.

Batch C1 moved `gpu_record.rs`, `gpu_frames.rs`, `gpu_restart.rs`, `frame_scaler.rs`, `frame_chain.rs` and `target_bounds.rs` into `platform/windows/capture/` - `gpu/record.rs`, `gpu/frames.rs`, `gpu/restart.rs`, `gpu/fit.rs`, `gpu/chain.rs` and `target.rs` - and left two `#[cfg(windows)]` module re-exports here so `switch_display.rs` could keep naming them. Batch D deleted the re-exports: this group now reaches the capture only through `ports::capture`, so nothing under `record/` is conditional on a platform and the whole group compiles for any target. `frame_fit.rs` did not move: its geometry is pure, unit-tested and shared with the export.

## Notify

```rust
pub type Notify = Arc<dyn Fn(&str) + Send + Sync>;
```

A one-way notification from a recording thread to the app: the reason string is what the HUD shows the user. *Why an `Arc<dyn Fn>` and not an `AppHandle`:* the capture/encode pipeline then carries no Tauri types, stays constructible in tests, and the one place that knows about events is `emit`. Two are built per recording - `record-warning` (a degraded but still-running take, e.g. an audio input that would not open) and `record-ended-early` (the OS ended the capture, or - on the legacy ffmpeg path only - its dimensions changed mid-record; `CAPTURE_CLOSED` and `DISPLAY_CHANGED` are the two reason strings sent through the latter).

## Level

```rust
pub type Level = Arc<dyn Fn(f32) + Send + Sync>;
```

A repeating audio-level report from a capture thread to the HUD's meter: the 0..1 RMS of the loudest block since the previous report, roughly every `recorder_threads::LEVEL_POLL_MS`. Same `Arc<dyn Fn>` shape and the same reason as `Notify` - the capture side keeps no Tauri types. Two are built per recording, one per source; `emit::level_emitter` is the only thing that turns one into an `audio-level` event.

## CAPTURE_CLOSED

```rust
pub const CAPTURE_CLOSED: &str = "The recorded window or display closed. The recording was saved up to that point.";
```

The reason passed to the capture-ended `Notify` when the OS - not the user - ends the capture: the recorded window was closed, or the recorded display was unplugged/disabled/slept. Both capture paths report it with the same wording (`platform::windows::capture::gpu::frames::Cap::on_closed` and the ffmpeg video thread in `platform::windows::capture::legacy::start_ffmpeg`), so the HUD has exactly one message to show.

## DISPLAY_CHANGED

```rust
pub const DISPLAY_CHANGED: &str = "Display changed - recording saved up to the change.";
```

The reason passed to the same `Notify` when the capture's OWN dimensions change mid-record (finding H1) - a recorded window maximized/restored/snapped, or a recorded display changed resolution, rotated, or was docked/undocked. Sent by the LEGACY ffmpeg path only. Its rawvideo pipe is sized once, at start, so it cannot keep encoding: it used to silently discard every frame from that instant on (`FfmpegFrameSink::write_or_skip`'s `Ok(false)` skip, forever) and now ends the take on the FIRST mismatched frame instead - `recording_session::RecordingSession::pump_once` latches its `mismatched` flag, which `platform::windows::capture::legacy::start_ffmpeg` reads. This distinct wording is what tells the HUD (and the user) it was a size change, not a closed window or display. The default GPU path no longer sends it at all: `gpu::frames::Cap::on_frame_arrived` fits a resized frame into the encoder's fixed canvas (`gpu::fit`) and keeps recording.

## recorder

Recording state plus the Start/Pause/Resume commands. Owns a `Mutex<Option<Running>>` as managed state; each command locks briefly, modifies or consumes the `Running` value, and returns. Key items: `Recorder` (managed-state singleton, with the `stopping` flag that keeps a start out of a stop's teardown window), `Running` (holds all live resources for one session, as four trait objects since Batch D), `TakeSpec` (what a take is asked for, base folder included), `start_take` / `pause_take` / `resume_take` (the take lifecycle, taking `&Platform` and `&Recorder` and no Tauri types), and the three `#[tauri::command]` wrappers `start_recording` / `pause_recording` / `resume_recording` over them.

## emit

The group's frontend event bridges and the bundle they are handed over as, split out of `recorder.rs` (at its line cap) when the level feed was added. Key items: `TakeHooks` (the four callbacks one take reports through, with `from_app` and the test-only `silent`), `emitter` (wraps an `AppHandle` in a `Notify` for `record-warning` / `record-ended-early`), `level_emitter` (wraps one in a `Level` that emits `audio-level`, tagged with its source).

## recorder_stop

The Stop half of the command surface. Key items: `stop_recording` (the `async` + `spawn_blocking` command), `stop_blocking` (its body, resolving the `Recorder` from the `AppHandle`), `stop_take` (signals all threads, joins in dependency order, saves inputs, finalizes the video, then writes `sync.json` / `project.tcursor` / recents from whatever was recorded - even when the finalize failed), `RecordingResult` (returned to the frontend).

## close_guard

The `CloseRequested` safety net (task-6, ruling R6): if the main window tries to close while `Recorder::is_busy()`, `lib.rs`'s `on_window_event` guard prevents it and hands off here. Key items: `finish_and_close` (finalizes the take directly via `recorder_stop::stop_recording` if nobody has claimed the stop yet, otherwise waits for whoever did, then closes the window - deliberately independent of the frontend, so the OS close button / Alt+F4 / a wedged renderer can never lose a take).

## recorder_threads

Thread-spawning helpers and persistence logic factored out of `recorder.rs` to keep that file under its line cap. Key items: `save_inputs` (stops the three input ports and writes `events.json`, `actions.json`, `typing.json`, `cursor.json` before the video thread is joined), `save_session_files` (`sync.json` + `project.tcursor` + recents), `spawn_mic_thread` / `spawn_system_thread` (spawn the audio holder threads; the loopback device is handed in from the audio port rather than opened here), `poll_until_stopped` (the shared 50ms stop-poll loop, which is also what reports each source's level), `LEVEL_POLL_MS`, `audio_warning` (the `record-warning` message for an input that would not open).

## segments

The ledger of mid-take source switches (2026-09-14): the extra mic and webcam segments a take produced and the display switches it made, filled by the `switch_mic`, `mark_webcam_segment` and `switch_display` commands and written into `sync.json` at Stop. See `segments.md`.

## switch_mic

The `switch_mic` command: change the running take's microphone (or turn it off) by ending the current mic thread, joining it so its WAV is finalized, and spawning the next onto `mic_2.wav`, `mic_3.wav`... Each one is logged as a `Segment` and merged back into the single `mic.wav` by `export::preview::segments_audio` during preprocess. See `switch_mic.md`.

## webcam_segments

The camera half of mid-take source switching: `webcam_segment_name` (which file a chunk with a given segment index belongs in, shared by `commands::append_webcam` and the command below) and the `mark_webcam_segment` command (stamps where segment `n` starts on the recording clock, into `Running.segments.webcam`). See `webcam_segments.md`.

## recording_session

Thin state machine wrapping one `FrameSource` and one `FrameSink`. Runs entirely on the `"video"` thread; has no threading primitives of its own. Key items: `RecordingSession` (owns source and sink, tracks frame count, timestamps, and whether a dimension mismatch ended the take), `RecordingSession::new`, `RecordingSession::pump_once` (pulls one frame and pushes it to the encoder at its pause-compressed timestamp; stops - and latches `dimension_mismatch()` - on the FIRST dimension-mismatched frame instead of skipping it forever), `RecordingSession::run` (variable-FPS loop with pause support, splitting the sink on each resume), `RecordingSession::run_paced` (delegates to `pacing::run_paced` for CFR mode), `RecordingSession::stop_and_finalize` (flushes the sink and returns the frame count), `RecordingSession::dimension_mismatch`, `SessionState` (lifecycle enum).

## frame_fit

Pure geometry for the GPU path's resize fit, split out so it is testable without live WGC or a GPU. Key items: `letterbox` (the centred, aspect-preserving destination rect for a source of one size inside a canvas of another).

## switch_display

The `switch_display` command: restarts the capture on a new target and installs the `events::remap::Remap` that keeps every later mouse sample landing where the pixels are. See `switch_display.md`.

## video_sink

What is left of the video sink in the portable tree: `VideoStopped` alone - frames + timestamps + an optional finalize error, so a failure never costs the session files. It stays here because it is `ports::capture::VideoSink::stop`'s return type. The pipeline itself lives in `platform/windows/capture/`, and Batch D deleted the `#[cfg(windows)]` re-exports that named it from here. See `video_sink.md`.

## pause_clock

Shared by `platform::windows::capture::gpu::frames::Cap` and `recording_session::RecordingSession`. Key items: `PauseClock` (places each arriving frame on the recording clock from the shared `PauseTotals` ledger), `FrameTick` (that placement: the `sync.json` timestamp and the matching encoder PTS, from one ledger read).

## pause_totals

The exact-span paused-time ledger, stamped at the pause/resume instants under the recorder lock and shared by every input tracker AND both capture paths. Key items: `PauseTotals`, `PauseTotals::pause` / `resume` / `elapsed_paused`, `PauseTotals::stamp_ms` (the one implementation of "remove the paused time"), `PauseTotals::stamp` (its `u32` form for input trackers).
