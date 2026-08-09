# src-tauri/src/session/record/recorder.rs

Top-level recording controller. Owns one `Mutex<Option<Running>>` shared with Tauri as managed state; each command locks it briefly and returns, so the mutex is never held during blocking I/O. The actual video encoding, mic capture, and system-audio loopback each run on their own `std::thread`.

## Running

```rust
struct Running {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    paused_totals: Arc<PauseTotals>,
    clock: Arc<dyn Clock>,
    video: VideoSink,
    mic_thread: Option<JoinHandle<()>>,
    system_thread: Option<JoinHandle<()>>,
    mouse: Option<MouseTracker>,
    keyboard: Option<KeyboardTracker>,
    cursor: Option<CursorTypeTracker>,
    events_path: PathBuf,
    actions_path: PathBuf,
    typing_path: PathBuf,
    cursor_path: PathBuf,
    screen: ScreenInfo,
    started_unix_ms: u64,
    events_ms: u64,
    mic_start: Arc<AtomicU64>,
    system_start: Arc<AtomicU64>,
    folder: String,
}
```

Private struct holding every live resource for one recording. Consumed in full by `stop_recording`.

- `stop: Arc<AtomicBool>` - shared shutdown flag polled by the video, mic, and system-audio threads. *Why Arc:* each thread clone needs ownership; SeqCst writes in `stop_recording` ensure all threads see the flag before `join`.
- `paused: Arc<AtomicBool>` - shared pause flag. *Why separate from stop:* pause is reversible; stop is terminal. The video encoding and audio threads both check this flag independently.
- `paused_totals: Arc<PauseTotals>` - the exact-span paused-time ledger (see `pause_totals.rs`), shared with every input tracker (mouse, keyboard, cursor-type). *Why separate from `paused`:* the `bool` flag is what capture threads poll; the ledger additionally records the exact ms each pause/resume happened, which those threads' own raw elapsed-time readings can't reconstruct on their own.
- `clock: Arc<dyn Clock>` - the same `Clock` used to derive `events_ms` and start the video/audio threads. *Why stored:* `pause_recording` / `resume_recording` need it to stamp `paused_totals` at the exact instant `paused` flips.
- `video: VideoSink` - the live recording video pipeline: GPU-native Media Foundation by default (no readback), else the legacy ffmpeg fallback (see `video_sink.rs`). *Why an enum:* `stop_recording` stops + finalizes it uniformly via `stop_and_collect`, which returns the frame count + per-frame capture timestamps regardless of which path ran.
- `mic_thread / system_thread: Option<JoinHandle<()>>` - `None` when mic/system-audio is off. *Why Option:* join is skipped cheaply for the disabled case.
- `mouse / keyboard / cursor: Option<...>` - active input trackers; `None` when not started. `cursor` is only `Some` for `CursorStyle::Enhanced`. *Why Option:* the `save_inputs` helper skips each absent tracker individually.
- `events_path / actions_path / typing_path / cursor_path: PathBuf` - destination paths passed to `save_inputs`. *Why stored here not in paths:* `Running` outlives the local `ProjectPaths` variable in `start_recording`.
- `screen: ScreenInfo` - capture dimensions and origin, needed by `EventLog`. *Why captured at start:* display resolution could change after recording begins; the log must reflect the resolution actually used.
- `started_unix_ms: u64` - Unix epoch time (ms) when recording started. *Why:* stored in `EventLog` so exports can correlate events to wall-clock time.
- `events_ms: u64` - clock value (ms) at the moment `MouseTracker` was started. *Why:* mouse event `t` values are relative to this origin; `sync.json` records it so the exporter can realign events.
- `mic_start / system_start: Arc<AtomicU64>` - stamped by each audio thread at their first real sample. *Why AtomicU64:* read back in `stop_recording` from the main thread without a lock; SeqCst load ensures the written value is visible.
- `folder: String` - project folder path, returned in `RecordingResult` and used to write `sync.json`.

### Used by

- `src-tauri/src/session/record/recorder.rs` (only) - created in `start_recording`, read in `pause_recording` / `resume_recording`, consumed in `stop_recording`.

## Recorder

```rust
#[derive(Default)]
pub struct Recorder {
    inner: Mutex<Option<Running>>,
}
```

Tauri managed-state singleton. `Default` is implemented so Tauri's `manage` call requires no manual construction.

- `inner: Mutex<Option<Running>>` - `None` when idle; `Some(Running)` between a successful `start_recording` and `stop_recording`. *Why Mutex:* the three Tauri commands share this state across the Tauri thread pool; the mutex prevents concurrent starts or a stop racing a pause.

### Used by

- `src-tauri/src/lib.rs` - registered as managed state via `.manage(Recorder::default())`.

## RecordingResult

```rust
#[derive(Serialize)]
pub struct RecordingResult { pub folder: String, pub frames: u64 }
```

Returned by `stop_recording` to the frontend.

- `folder: String` - absolute path to the project directory. *Why:* the frontend passes this path back to `export_project` and the editor commands.
- `frames: u64` - total successfully encoded video frames. *Why:* informational for the UI; also useful when diagnosing a drop in frame count.

### Used by

- `src-tauri/src/session/record/recorder.rs` - constructed and returned by `stop_recording`.

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
- `recorder: tauri::State<'_, Recorder>` - the singleton managed state. *Why State injection:* Tauri resolves it from the app's managed-state registry, avoiding global variables.
- `app: tauri::AppHandle` (Task 39) - resolves the main window to swap in the REC-lit icon once recording actually starts (`win::sys::brand_icon::set_recording`). Brand flair only - never fails the command.

### Returns

`Ok(String)` (the project folder) on success. `Err(String)` for: already recording, folder creation failure, `WgcFrameSource` init failure, or ffmpeg encoder spawn failure. On any `start_video` error, the mic/system-audio threads spawned earlier in this call are stopped and joined first (see step 6), so a failed start never leaks an open audio device or a running thread. The returned folder lets the HUD stream the webcam into it during recording.

### Implementation

1. Lock `recorder.inner`. Return `Err("already recording")` if `Some`. *Why check before any I/O:* prevents double-start from a fast frontend double-click with no wasted setup.
2. Resolve `base = dirs_next::video_dir() / "TCursor"`. Create `ProjectPaths`, call `ensure()`. *Why `dirs_next::video_dir`:* platform-native; falls back to `temp_dir` so recording never fails for lack of a video directory.
3. Snapshot settings via `settings::store::load` and write to `paths.settings()`. *Why snapshot at start:* the user might change settings mid-session; the export always uses the settings active at record time, ensuring reproducibility.
4. Query `primary_refresh_hz`, capped to 60. *Why cap:* 60fps is the practical ceiling for current targets; higher refresh rates would waste encoder bandwidth.
5. Create the `paused_totals: Arc<PauseTotals>` ledger. Start `MouseTracker`, `KeyboardTracker`, and (if Enhanced style) `CursorTypeTracker`, each given a clone so their stamp sites can pause-adjust their raw elapsed-ms readings. Start `spawn_mic_thread` and `spawn_system_thread`. *Why before the video pipeline:* building the encoder (first ffmpeg probe / Media Foundation setup) can take a moment. Starting inputs first ensures they capture from t=0 and do not miss that startup gap.
6. Call `video_sink::start_video(game_mode, .., target_id.as_deref(), ..)` - the slow part - wrapped in a `match` (not `?`). It builds the GPU-native `GpuRecorder` (Media Foundation, no readback) by default, or the legacy ffmpeg pipe when `game_mode` (the compatibility toggle) is set or the GPU encoder fails to init. Returns the `VideoSink` and the captured `(w, h, origin_x, origin_y)`. *Why after inputs:* see step 5; the slow part is intentionally last. *Why `match` not `?`:* on `Err`, `stop` is stored `true` (`SeqCst`) and `mic_thread`/`system_thread` are joined before returning the error - otherwise those threads' 50ms poll loops would never see a stop signal (it is only ever set from `Running`, which is never constructed on this path) and would run forever, leaking the thread and leaving the mic/loopback device open with its WAV never finalized. `mouse`/`keyboard`/`cursor` need no equivalent handling: they are plain locals at this point (not yet moved into `Running`), so returning early drops them, and their `Drop` impls already stop the underlying hooks.
7. Build `ScreenInfo` from `(w, h, origin_x, origin_y)`, stamp `started_unix_ms`, and store everything in `Running` (including `video: VideoSink`); assign to `*guard`.
8. Call `brand_icon::set_recording(&app, true)` (Task 39) - swaps the main window's icon to the REC-lit variant. After the guard is set, so it only fires on an actual successful start.

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
2. Call `r.paused_totals.pause(r.clock.now_ms())`. *Why before the flag flip, under the same lock:* the ledger and the flag must agree on the exact pause instant; reading the clock here (rather than inside each tracker thread) is what makes this an exact-span model instead of the tick-based `PauseClock`.
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
2. Call `r.paused_totals.resume(r.clock.now_ms())`. *Why:* folds the just-ended pause span into the ledger's accumulated total before any tracker reads `elapsed_paused` again.
3. Store `false` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* same reasoning as `pause_recording`; the clearing write must be globally visible before the threads' next reads.

## stop_recording

```rust
#[tauri::command]
pub fn stop_recording(recorder: tauri::State<'_, Recorder>, app: tauri::AppHandle) -> Result<RecordingResult, String>
```

Signals all threads to stop, joins them in dependency order, persists input data, `sync.json`, and `project.tcursor`, and returns the `RecordingResult`.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.
- `app: tauri::AppHandle` (Task 39) - resolves the main window to swap the icon back to normal. Brand flair only - never fails the command.

### Returns

`Ok(RecordingResult)` with the project folder and frame count. `Err(String)` if the video thread panicked or `stop_and_finalize` failed.

### Implementation

1. Lock `recorder.inner` and `take` the `Running`. Return `Err("not recording")` if `None`. *Why `take`:* consumes the `Running`, making the state `None` so a subsequent `start_recording` is permitted.
2. Call `brand_icon::set_recording(&app, false)` (Task 39) - only reached once step 1 confirms a recording was actually taken, so a redundant Stop (already-idle) never touches the icon.
3. Set `stop = true` (SeqCst) - signals the audio threads (the video pipeline is stopped below).
4. Join `mic_thread` and `system_thread`. *Why audio first:* lightweight (50ms loop), they finish quickly.
5. Call `save_inputs`. *Why before the video stop:* if finalizing the video errors, the `?` would skip `save_inputs` and lose the events/actions; saving first guarantees they persist.
6. `running.video.stop_and_collect()` - stop + finalize the video pipeline (GPU: end capture + `encoder.finish()`; ffmpeg: set halt + WM_QUIT to unblock the WGC thread + join), returning `(frames, frame_ts)`. Propagate errors as `Err(String)`.
7. Build `SyncLog` from `frame_ts`, `events_ms`, and the atomic audio start times (0 treated as absent). Save to `folder/sync.json`. *Why after the video stop:* `frame_ts` is only complete once the pipeline has finalized.
8. Build a `project::manifest::ProjectManifest` from `running.screen.w/h` (`preprocessed: false`) and save it to `paths.manifest()` (`folder/project.tcursor`), then call `project::recents::touch(&running.folder)`. Both best-effort (`eprintln!`/silently swallowed on failure) - a write failure here must never fail the recording, since the folder is already a fully valid project without them. `preprocessed` starts `false` here regardless - the frontend calls `export::preview::preprocess::preprocess_project` right after this command resolves (shown as the HUD's "Saving..." progress via `useRecordingFlow`) and that flips it once its pass finishes. This is NOT done as a detached background thread from `stop_recording` itself anymore (that used to race the editor's mount - the proxy/thumbs/waveform transcode could still be running when the editor opened, which was exactly the "preview still takes a while to load" lag); awaiting it with progress in the caller fixes that.
9. Return `RecordingResult { folder, frames }`.
