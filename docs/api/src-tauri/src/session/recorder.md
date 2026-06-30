# src-tauri/src/session/recorder.rs

Top-level recording controller. Owns one `Mutex<Option<Running>>` shared with Tauri as managed state; each command locks it briefly and returns, so the mutex is never held during blocking I/O. The actual video encoding, mic capture, and system-audio loopback each run on their own `std::thread`.

## Running

```rust
struct Running {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    video_halt: Arc<AtomicBool>,
    video_stopper: Option<Box<dyn FnOnce() + Send>>,
    video_thread: JoinHandle<std::io::Result<(u64, Vec<u64>)>>,
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
- `video_halt: Arc<AtomicBool>` - WGC-specific halt flag obtained from `FrameSource::halt_handle`. *Why a second bool:* WGC requires its own tear-down signal distinct from the generic `stop` used by the recording loop.
- `video_stopper: Option<Box<dyn FnOnce() + Send>>` - optional callback that invokes the WGC session's OS-level stop. *Why Option:* `take_stopper` may not produce one on all platforms; the field is consumed once in `stop_recording`.
- `video_thread: JoinHandle<io::Result<(u64, Vec<u64>)>>` - the encoding thread; returns frame count and frame timestamps on success. *Why io::Result:* `FrameSink::finish` can fail (e.g. ffmpeg crash); the error is propagated to the frontend via `stop_recording`.
- `mic_thread / system_thread: Option<JoinHandle<()>>` - `None` when mic/system-audio is off. *Why Option:* join is skipped cheaply for the disabled case.
- `mouse / keyboard / cursor: Option<...>` - active input trackers; `None` when not started. `cursor` is only `Some` for `CursorStyle::Enhanced`. *Why Option:* the `save_inputs` helper skips each absent tracker individually.
- `events_path / actions_path / typing_path / cursor_path: PathBuf` - destination paths passed to `save_inputs`. *Why stored here not in paths:* `Running` outlives the local `ProjectPaths` variable in `start_recording`.
- `screen: ScreenInfo` - capture dimensions and origin, needed by `EventLog`. *Why captured at start:* display resolution could change after recording begins; the log must reflect the resolution actually used.
- `started_unix_ms: u64` - Unix epoch time (ms) when recording started. *Why:* stored in `EventLog` so exports can correlate events to wall-clock time.
- `events_ms: u64` - clock value (ms) at the moment `MouseTracker` was started. *Why:* mouse event `t` values are relative to this origin; `sync.json` records it so the exporter can realign events.
- `mic_start / system_start: Arc<AtomicU64>` - stamped by each audio thread at their first real sample. *Why AtomicU64:* read back in `stop_recording` from the main thread without a lock; SeqCst load ensures the written value is visible.
- `folder: String` - project folder path, returned in `RecordingResult` and used to write `sync.json`.

### Used by

- `src-tauri/src/session/recorder.rs` (only) - created in `start_recording`, read in `pause_recording` / `resume_recording`, consumed in `stop_recording`.

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

- `src-tauri/src/session/recorder.rs` - constructed and returned by `stop_recording`.

## start_recording

```rust
#[tauri::command]
pub fn start_recording(
    project_name: String,
    mic_id: Option<String>,
    system_audio: bool,
    game_mode: bool,
    recorder: tauri::State<'_, Recorder>,
) -> Result<(), String>
```

Creates the project folder, starts all input trackers and audio threads, spawns the video encoding thread, and stores the live session in `Recorder`.

### Inputs

- `project_name: String` - subfolder name under `Videos/TCursor`. *Why caller-supplied:* the frontend generates a timestamp string so the name is predictable and unique without coordination.
- `mic_id: Option<String>` - cpal device name, or `None` to skip mic. *Why Option:* mic is optional; `spawn_mic_thread` converts `None` to an immediate `None` return.
- `system_audio: bool` - whether to capture loopback audio. *Why bool not Option:* simple on/off; no device selection is needed for loopback.
- `game_mode: bool` - if true, runs the video thread in CFR mode via `session.run_paced`. *Why separate flag:* game-mode uses a different time model (uniform timestamps from `pacing.rs`) and should be an explicit user choice.
- `recorder: tauri::State<'_, Recorder>` - the singleton managed state. *Why State injection:* Tauri resolves it from the app's managed-state registry, avoiding global variables.

### Returns

`Ok(())` on success. `Err(String)` for: already recording, folder creation failure, `WgcFrameSource` init failure, or ffmpeg encoder spawn failure.

### Implementation

1. Lock `recorder.inner`. Return `Err("already recording")` if `Some`. *Why check before any I/O:* prevents double-start from a fast frontend double-click with no wasted setup.
2. Resolve `base = dirs_next::video_dir() / "TCursor"`. Create `ProjectPaths`, call `ensure()`. *Why `dirs_next::video_dir`:* platform-native; falls back to `temp_dir` so recording never fails for lack of a video directory.
3. Snapshot settings via `settings::store::load` and write to `paths.settings()`. *Why snapshot at start:* the user might change settings mid-session; the export always uses the settings active at record time, ensuring reproducibility.
4. Query `primary_refresh_hz`, capped to 60. *Why cap:* 60fps is the practical ceiling for current targets; higher refresh rates would waste encoder bandwidth.
5. Start `WgcFrameSource`. *Why before the encoder:* the source reports `(w, h)` needed by the encoder constructor; also starts OS capture at the same moment as the other inputs.
6. Start `MouseTracker`, `KeyboardTracker`, and (if Enhanced style) `CursorTypeTracker`. Start `spawn_mic_thread` and `spawn_system_thread`. *Why before the encoder:* `FfmpegFrameSink::new` can take seconds on first run (ffmpeg probe). Starting inputs first ensures they capture from t=0 and do not miss that long startup gap.
7. Construct the `FfmpegFrameSink`: `new_vfr` for normal recording (VFR - frames stamped with their real arrival time, so `video.mp4` plays at true speed even when the capture rate dips below the display refresh) or `new` for game mode (fixed CFR, since `run_paced` paces the source to that rate). *Why after inputs:* see step 6; the slow part is intentionally last. The export is unaffected by VFR - it times frames by `sync.json`, not the file's PTS.
8. Obtain `video_halt` and `video_stopper` from the source before the source is moved into the video thread.
9. Spawn the `"video"` thread: create `RecordingSession`, call `run_paced` or `run`, then `stop_and_finalize`. *Why its own thread:* the encoding loop is blocking; it must not run on the Tauri command thread.
10. Store all resources in `Running`, assign to `*guard`.

## pause_recording

```rust
#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

Sets the shared `paused` flag to `true` with `SeqCst` ordering.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Store `true` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* ensures the video, mic, and system-audio threads observe the flag before the next iteration of their respective sleep loops (2-50ms latency is acceptable; SeqCst removes any reorder uncertainty).

## resume_recording

```rust
#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

Clears the shared `paused` flag, resuming all capture threads.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Store `false` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* same reasoning as `pause_recording`; the clearing write must be globally visible before the threads' next reads.

## stop_recording

```rust
#[tauri::command]
pub fn stop_recording(recorder: tauri::State<'_, Recorder>) -> Result<RecordingResult, String>
```

Signals all threads to stop, joins them in dependency order, persists input data and `sync.json`, and returns the `RecordingResult`.

### Inputs

- `recorder: tauri::State<'_, Recorder>` - the singleton managed state.

### Returns

`Ok(RecordingResult)` with the project folder and frame count. `Err(String)` if the video thread panicked or `stop_and_finalize` failed.

### Implementation

1. Lock `recorder.inner` and `take` the `Running`. Return `Err("not recording")` if `None`. *Why `take`:* consumes the `Running`, making the state `None` so a subsequent `start_recording` is permitted.
2. Set `video_halt = true` (SeqCst), call `video_stopper()` if present. *Why `video_halt` first:* WGC needs its own OS-level teardown signal before the generic `stop` flag; calling them in this order avoids a race where the capture loop races past `stop` before WGC finishes.
3. Set `stop = true` (SeqCst). All threads are now converging to termination.
4. Join `mic_thread` and `system_thread`. *Why join audio before video:* audio threads are lightweight (50ms sleep loop) and finish quickly; joining them first drains any buffered audio samples.
5. Call `save_inputs`. *Why before the video join:* if `stop_and_finalize` returns an error, the `?` operator in the video join would propagate it and skip the `save_inputs` call; input data would be lost. Saving first ensures events and actions are always persisted.
6. Join `video_thread`. Propagate thread panic or `io::Result` errors as `Err(String)`.
7. Build `SyncLog` from `frame_ts`, `events_ms`, and the atomic audio start times (0 treated as absent). Save to `folder/sync.json`. *Why after the video join:* `frame_ts` is only complete once `stop_and_finalize` has returned.
8. Spawn a detached background thread calling `export::thumbs::prewarm(folder)` to eagerly generate the editor's proxy/thumbnails/waveforms/preview-audio. *Why here, off-thread:* capture has fully stopped (all threads joined), so this heavy ffmpeg work cannot compete with the live capture; doing it now makes opening the editor instant instead of transcoding on open. Fire-and-forget - `stop_recording` returns immediately and the editor's lazy `ensure_*` re-attempts anything not yet finished.
9. Return `RecordingResult { folder, frames }`.
