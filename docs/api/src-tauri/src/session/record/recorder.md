# src-tauri/src/session/record/recorder.rs

Recording state plus the Start/Pause/Resume commands. Owns one `Mutex<Option<Running>>` shared with Tauri as managed state; each command locks it briefly and returns, so the mutex is never held during blocking I/O. The actual video encoding, mic capture, and system-audio loopback each run on their own `std::thread`. The Stop command lives in `recorder_stop.rs` (see `recorder_stop.md`).

## Running

```rust
pub(super) struct Running {
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub paused_totals: Arc<PauseTotals>,
    pub clock: Arc<dyn Clock>,
    pub video: VideoSink,
    pub mic_thread: Option<JoinHandle<()>>,
    pub system_thread: Option<JoinHandle<()>>,
    pub mouse: Option<MouseTracker>,
    pub keyboard: Option<KeyboardTracker>,
    pub cursor: Option<CursorTypeTracker>,
    pub events_path: PathBuf,
    pub actions_path: PathBuf,
    pub typing_path: PathBuf,
    pub cursor_path: PathBuf,
    pub screen: ScreenInfo,
    pub started_unix_ms: u64,
    pub events_ms: u64,
    pub mic_start: Arc<AtomicU64>,
    pub system_start: Arc<AtomicU64>,
    pub folder: String,
}
```

Holds every live resource for one recording. Consumed in full by `recorder_stop::stop_blocking`. `pub(super)` with `pub` fields so that sibling module can consume it while nothing outside `record` can see it.

- `stop: Arc<AtomicBool>` - shared shutdown flag polled by the video, mic, and system-audio threads. *Why Arc:* each thread clone needs ownership; SeqCst writes in `stop_blocking` ensure all threads see the flag before `join`. Also read by the ffmpeg video thread to tell a user Stop apart from the OS ending the capture.
- `paused: Arc<AtomicBool>` - shared pause flag. *Why separate from stop:* pause is reversible; stop is terminal. The video encoding and audio threads both check this flag independently.
- `paused_totals: Arc<PauseTotals>` - the exact-span paused-time ledger (see `pause_totals.rs`), shared with every input tracker (mouse, keyboard, cursor-type) AND, since the C1 fix, with both capture paths through `PauseClock`. *Why separate from `paused`:* the `bool` flag is what capture threads poll; the ledger additionally records the exact ms each pause/resume happened, which those threads' own frame-arrival readings cannot reconstruct.
- `clock: Arc<dyn Clock>` - the same `Clock` used to derive `events_ms` and start the video/audio threads. *Why stored:* `pause_recording` / `resume_recording` need it to stamp `paused_totals` at the exact instant `paused` flips.
- `video: VideoSink` - the live recording video pipeline: GPU-native Media Foundation by default (no readback), else the legacy ffmpeg fallback (see `video_sink.rs`). *Why an enum:* the stop path finalizes it uniformly via `stop_and_collect`, which returns a `VideoStopped` regardless of which path ran.
- `mic_thread / system_thread: Option<JoinHandle<()>>` - `None` when mic/system-audio is off. *Why Option:* join is skipped cheaply for the disabled case.
- `mouse / keyboard / cursor: Option<...>` - active input trackers; `None` when not started. `cursor` is now `Some` on EVERY take, whatever the style: capture excludes the OS cursor for all of them, so the shape track AND the captured cursor layer both have to be sampled live for any style to be selectable in the editor afterwards. *Why still Option:* the `save_inputs` helper skips each absent tracker individually, and a spawn failure or a minimal build can legitimately have none.
- `events_path / actions_path / typing_path / cursor_path: PathBuf` - destination paths passed to `save_inputs`. *Why stored here not in paths:* `Running` outlives the local `ProjectPaths` variable in `start_recording`.
- `screen: ScreenInfo` - capture dimensions and origin, needed by `EventLog`. *Why captured at start:* display resolution could change after recording begins; the log must reflect the resolution actually used.
- `started_unix_ms: u64` - Unix epoch time (ms) when recording started. *Why:* stored in `EventLog` so exports can correlate events to wall-clock time.
- `events_ms: u64` - clock value (ms) at the moment `MouseTracker` was started. *Why:* mouse event `t` values are relative to this origin; `sync.json` records it so the exporter can realign events.
- `mic_start / system_start: Arc<AtomicU64>` - stamped by each audio thread at their first real sample. *Why AtomicU64:* read back in the stop path without a lock; SeqCst load ensures the written value is visible.
- `folder: String` - project folder path, returned in `RecordingResult` and used to write `sync.json`.

### Used by

- `src-tauri/src/session/record/recorder.rs` - created in `start_recording`, read in `pause_recording` / `resume_recording`.
- `src-tauri/src/session/record/recorder_stop.rs` - consumed in `stop_blocking`.

## Recorder

```rust
#[derive(Default)]
pub struct Recorder {
    pub(super) inner: Mutex<Option<Running>>,
    pub(super) stopping: AtomicBool,
}
```

Tauri managed-state singleton. `Default` is implemented so Tauri's `manage` call requires no manual construction.

- `inner: Mutex<Option<Running>>` - `None` when idle; `Some(Running)` between a successful `start_recording` and the stop path taking it. *Why Mutex:* the four Tauri commands share this state across the Tauri thread pool; the mutex prevents concurrent starts or a stop racing a pause.
- `stopping: AtomicBool` - `true` while `stop_blocking` is tearing a session down. *Why needed at all:* the teardown runs OUTSIDE the lock (it joins threads and finalizes a possibly multi-GB MP4 - holding the recorder lock across that would block Pause/Resume on the main thread), and `Running` has already been taken by then, so `inner.is_some()` alone would wave a concurrent `start_recording` straight through. `MouseTracker`'s hook sink is a process-global (`events::track::tracker::SINK`) that a start overwrites unconditionally, so that interleave writes the finished take's `events.json` from the NEW empty collector and leaves the new take's hook with no sink at all - zero mouse events, so no cursor, no click FX and no auto-zoom for its whole duration. *Why an atomic rather than a second mutex:* it is only ever read and written while holding `inner`'s lock, so it is effectively part of that guarded state and cannot introduce a lock-order cycle.

### Used by

- `src-tauri/src/lib.rs` - registered as managed state via `.manage(Recorder::default())`; its `CloseRequested` guard also calls `is_busy` directly on the `State` it resolves.
- `src-tauri/src/session/record/recorder_stop.rs` - resolved from the `AppHandle` in `stop_blocking`.
- `src-tauri/src/session/record/close_guard.rs` - `finish_and_close` calls `is_recording` and `is_busy`.

## Recorder::is_recording

```rust
pub fn is_recording(&self) -> bool
```

True while a take is actively recording (`inner` is `Some`) - i.e. nobody has started stopping it yet.

### Returns

`self.inner.lock().is_some()` (poison-tolerant, same `unwrap_or_else(|e| e.into_inner())` pattern as every other lock site in this file).

### Used by

- `src-tauri/src/lib.rs` - the `CloseRequested` guard's `is_busy()` check folds this in.
- `src-tauri/src/session/record/close_guard.rs` - `finish_and_close` uses this to decide whether IT must run `stop_recording`, versus someone else already owning the stop.

## Recorder::is_busy

```rust
pub fn is_busy(&self) -> bool
```

True while a take is recording OR its stop is still finalizing.

### Returns

`is_recording() || stopping.load(SeqCst)`. *Why both:* `stop_blocking` takes `Running` out of `inner` (making `is_recording()` false) well before the finalize it then runs - joining audio threads, writing the video's `moov` atom, `sync.json`, the manifest - actually completes; `stopping` covers exactly that window (same flag `start_recording` already checks to keep a new start out of a stop's teardown - see `Recorder::stopping` above).

### Used by

- `src-tauri/src/lib.rs` - the `CloseRequested` guard: a close request landing anywhere in `is_busy()`'s window is prevented and handed to `close_guard::finish_and_close`, so the process can never exit mid-finalize (task-6 (a) / ruling R6).
- `src-tauri/src/session/record/close_guard.rs` - the wait loop when someone else already owns the stop.

## start_recording

```rust
#[tauri::command]
pub fn start_recording(
    project_name: String,
    mic_id: Option<String>,
    target_id: Option<String>,
    system_audio: bool,
    game_mode: bool,
    recorder: tauri::State<'_, Recorder>,
    app: tauri::AppHandle,
) -> Result<String, String>
```

Creates the project folder, starts all input trackers and audio threads, spawns the video encoding thread, stores the live session in `Recorder`, and returns the project folder path.

### Inputs

- `project_name: String` - subfolder name under `Videos/TCursor`. *Why caller-supplied:* the frontend generates a timestamp string so the name is predictable and unique without coordination.
- `mic_id: Option<String>` - cpal device name, or `None` to skip mic. *Why Option:* mic is optional; `spawn_mic_thread` converts `None` to an immediate `None` return.
- `target_id: Option<String>` - which display/window to capture, or `None` for the default target. Forwarded to `video_sink::start_video`.
- `system_audio: bool` - whether to capture loopback audio. *Why bool not Option:* simple on/off; no device selection is needed for loopback.
- `game_mode: bool` - if true, runs the video thread via the legacy ffmpeg fallback path (the compatibility toggle) instead of the GPU-native one. *Why separate flag:* an explicit user choice for setups where the GPU path has issues.
- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.
- `app: tauri::AppHandle` - source of the two `Notify` emitters and the two `Level` emitters (see `emit::emitter` / `emit::level_emitter`, both lifted out of this file when the level feed was added so it stayed under the line cap), and resolves the main window to swap in the REC-lit icon once recording actually starts (`win::sys::brand_icon::set_recording`). The icon is brand flair only - never fails the command.

### Returns

`Ok(String)` (the project folder) on success. `Err(String)` for: already recording (including the tail of a stop still in progress - see `Recorder::stopping`), folder creation failure, `WgcFrameSource` init failure, or ffmpeg encoder spawn failure. On any `start_video` error, the mic/system-audio threads spawned earlier in this call are stopped and joined first (see step 6), so a failed start never leaks an open audio device or a running thread. The returned folder lets the HUD stream the webcam into it during recording.

An audio input that fails to OPEN is not an error here - the take is still worth having - so it arrives as a `record-warning` event instead; see `recorder_threads::audio_warning`.

### Implementation

1. Lock `recorder.inner`. Return `Err("already recording")` if `Some` **or** if `stopping` is set. *Why check before any I/O:* prevents double-start from a fast frontend double-click with no wasted setup, and keeps a start out of the window where a previous stop is still detaching the global input hooks.
2. Resolve `base = dirs_next::video_dir() / "TCursor"`. Create `ProjectPaths`, call `ensure()`. *Why `dirs_next::video_dir`:* platform-native; falls back to `temp_dir` so recording never fails for lack of a video directory.
3. Snapshot settings via `settings::store::load` and write to `paths.settings()`. *Why snapshot at start:* the user might change settings mid-session; the export always uses the settings active at record time, ensuring reproducibility.
4. Query `primary_refresh_hz`, capped to 60. *Why cap:* 60fps is the practical ceiling for current targets; higher refresh rates would waste encoder bandwidth.
5. Create the `paused_totals: Arc<PauseTotals>` ledger and the `record-warning` emitter. Start `MouseTracker`, `KeyboardTracker`, and (if Enhanced style) `CursorTypeTracker`, each given a ledger clone so their stamp sites can pause-adjust their raw elapsed-ms readings. Start `spawn_mic_thread` and `spawn_system_thread`, each given the warning emitter and its own `level_emitter` (`"mic"` / `"system"`), which is what feeds the HUD's live wave meter - one `audio-level` event per open source every ~50ms, straight from the capture that is writing the WAV, so a meter can only ever move for audio this take is really recording. *Why before the video pipeline:* building the encoder (first ffmpeg probe / Media Foundation setup) can take a moment. Starting inputs first ensures they capture from t=0 and do not miss that startup gap.
6. Build a `VideoStart` (including the SAME `paused_totals` and a `record-ended-early` emitter) and call `video_sink::start_video` - the slow part - wrapped in a `match` (not `?`). It builds the GPU-native `GpuRecorder` (Media Foundation, no readback) by default, or the legacy ffmpeg pipe when `game_mode` is set or the GPU encoder fails to init. Returns the `VideoSink` and the captured `(w, h, origin_x, origin_y)`. `with_cursor` is hardcoded `false` - the OS cursor is NEVER baked into the pixels for any style, because it is captured as its own layer instead (`events::track::cursorlayer`); that is what makes the cursor style an editor decision rather than a record-time one. *Why the ledger goes to the video too:* that is the C1 fix - the encoded video's own PTS is compressed by the same ledger the input streams subtract, so `video.mp4` and `sync.json` share one clock. *Why `match` not `?`:* on `Err`, `stop` is stored `true` (`SeqCst`) and `mic_thread`/`system_thread` are joined before returning the error - otherwise those threads' 50ms poll loops would never see a stop signal (it is only ever set from `Running`, which is never constructed on this path) and would run forever, leaking the thread and leaving the mic/loopback device open with its WAV never finalized. `mouse`/`keyboard`/`cursor` need no equivalent handling: they are plain locals at this point, so returning early drops them, and their `Drop` impls already stop the underlying hooks.
7. Build `ScreenInfo` from `(w, h, origin_x, origin_y)`, stamp `started_unix_ms`, and store everything in `Running`; assign to `*guard`.
8. Call `brand_icon::set_recording(&app, true)` - swaps the main window's icon to the REC-lit variant. After the guard is set, so it only fires on an actual successful start.

## pause_recording

```rust
#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

Stamps `paused_totals` with the pause instant, then sets the shared `paused` flag to `true` with `SeqCst` ordering.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Call `r.paused_totals.pause(r.clock.now_ms())`. *Why before the flag flip, under the same lock:* the ledger and the flag must agree on the exact pause instant. Since C1/H2 this single reading is what BOTH the input streams and the video's own PTS are compressed by, so the whole recording removes exactly the same span.
3. Store `true` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* ensures the video, mic, and system-audio threads observe the flag before the next iteration of their respective sleep loops (2-50ms latency is acceptable; SeqCst removes any reorder uncertainty).

## resume_recording

```rust
#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

Closes the in-progress span on `paused_totals`, then clears the shared `paused` flag, resuming all capture threads.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Call `r.paused_totals.resume(r.clock.now_ms())`. *Why:* folds the just-ended pause span into the ledger's accumulated total before any tracker - or either capture path - reads `elapsed_paused` again.
3. Store `false` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* same reasoning as `pause_recording`; the clearing write must be globally visible before the threads' next reads.
