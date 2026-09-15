# src-tauri/src/session/record/recorder.rs

Recording state plus the Start/Pause/Resume commands. Owns one `Mutex<Option<Running>>` shared with Tauri as managed state; each command locks it briefly and returns, so the mutex is never held during blocking I/O. The actual video encoding, mic capture, and system-audio loopback each run on their own `std::thread`. The Stop command lives in `recorder_stop.rs` (see `recorder_stop.md`).

**The take lifecycle and its Tauri commands are separate since Batch D.** `start_take`, `pause_take` and `resume_take` take `&Platform` and `&Recorder` and name no Tauri type; `start_recording`, `pause_recording` and `resume_recording` are the `#[tauri::command]` wrappers that resolve the two states, build the `TakeHooks` from the `AppHandle` and (on start) swap the REC icon. That seam is what makes `platform/mock/cycle_tests.rs` possible - the first test that has ever started and stopped a real take.

## Running

```rust
pub(super) struct Running {
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub paused_totals: Arc<PauseTotals>,
    pub clock: Arc<dyn Clock>,
    pub video: Box<dyn VideoSink>,
    pub mic_thread: Option<JoinHandle<()>>, pub mic_stop: Arc<AtomicBool>,
    pub system_thread: Option<JoinHandle<()>>,
    pub mouse: Box<dyn PointerPort>,
    pub keyboard: Box<dyn HotkeyPort>,
    pub cursor: Box<dyn CursorShapePort>,
    pub events_path: PathBuf,
    pub actions_path: PathBuf,
    pub typing_path: PathBuf,
    pub cursor_path: PathBuf,
    pub screen: ScreenInfo,
    pub started_unix_ms: u64,
    pub events_ms: u64,
    pub mic_start: Arc<AtomicU64>,
    pub system_start: Arc<AtomicU64>,
    pub folder: String, pub segments: SharedSegments,
}
```

Holds every live resource for one recording. Consumed in full by `recorder_stop::stop_blocking`. `pub(super)` with `pub` fields so that sibling module can consume it while nothing outside `record` can see it.

- `stop: Arc<AtomicBool>` - shared shutdown flag polled by the video and system-audio threads. *Why Arc:* each thread clone needs ownership; SeqCst writes in `stop_take` ensure all threads see the flag before `join`. Also read by the ffmpeg video thread to tell a user Stop apart from the OS ending the capture, which is why Batch D put it in `CaptureRequest` rather than letting the sink own a private one - see `ports/capture.md`.
- `paused: Arc<AtomicBool>` - shared pause flag. *Why separate from stop:* pause is reversible; stop is terminal. The video encoding and audio threads both check this flag independently.
- `paused_totals: Arc<PauseTotals>` - the exact-span paused-time ledger (see `pause_totals.rs`), shared with every input tracker (mouse, keyboard, cursor-type) AND, since the C1 fix, with both capture paths through `PauseClock`. *Why separate from `paused`:* the `bool` flag is what capture threads poll; the ledger additionally records the exact ms each pause/resume happened, which those threads' own frame-arrival readings cannot reconstruct.
- `clock: Arc<dyn Clock>` - the same `Clock` used to derive `events_ms` and start the video/audio threads. *Why stored:* `pause_recording` / `resume_recording` need it to stamp `paused_totals` at the exact instant `paused` flips.
- `video: Box<dyn VideoSink>` - the live recording video pipeline behind `ports::capture::VideoSink`. On Windows that is GPU-native Media Foundation by default (no readback), else the legacy ffmpeg fallback; the recorder cannot tell and does not ask. *Why `Box` and not `Arc`:* `VideoSink::stop` takes `self: Box<Self>` because stopping consumes the pipeline, and an `Arc` cannot express that without a runtime unwrap. There is exactly one owner and one consumer.
- `mic_thread / system_thread: Option<JoinHandle<()>>` - `None` when mic/system-audio is off. *Why Option:* join is skipped cheaply for the disabled case.
- `mic_stop: Arc<AtomicBool>` - the CURRENT mic thread's own stop flag, distinct from the take-wide `stop`: `switch_mic` (mid-take source switching, 2026-09-14) stops just that thread, joins it so its WAV finalizes, and spawns the next with a fresh flag. `stop_blocking` sets both.
- `mouse / keyboard / cursor: Box<dyn PointerPort / HotkeyPort / CursorShapePort>` - the take's three input streams, built already running by `InputPort` and consumed by `save_inputs`. All three exist on EVERY take, whatever the cursor style: capture excludes the OS cursor for all of them, so the shape track AND the captured cursor layer both have to be sampled live for any style to be selectable in the editor afterwards. *Why not `Option` any more:* they were `Option` from when the cursor tracker was conditional, and every one of them has been unconditional since; Batch D dropped the wrapper rather than keep three branches in `save_inputs` that could not be taken. A platform that can start one stream and not another returns one that produces nothing, which is the degradation the ports are shaped for - three separate traits precisely so it is not all-or-nothing.
- `events_path / actions_path / typing_path / cursor_path: PathBuf` - destination paths passed to `save_inputs`. *Why stored here not in paths:* `Running` outlives the local `ProjectPaths` variable in `start_recording`.
- `screen: ScreenInfo` - capture dimensions and origin, needed by `EventLog`. *Why captured at start:* display resolution could change after recording begins; the log must reflect the resolution actually used.
- `started_unix_ms: u64` - Unix epoch time (ms) when recording started. *Why:* stored in `EventLog` so exports can correlate events to wall-clock time.
- `events_ms: u64` - clock value (ms) at the moment `MouseTracker` was started. *Why:* mouse event `t` values are relative to this origin; `sync.json` records it so the exporter can realign events.
- `mic_start / system_start: Arc<AtomicU64>` - stamped by each audio thread at their first real sample. *Why AtomicU64:* read back in the stop path without a lock; SeqCst load ensures the written value is visible.
- `folder: String` - project folder path, returned in `RecordingResult` and used to write `sync.json`.
- `segments: SharedSegments` - the ledger of mid-take source switches (`segments.rs`), taken at Stop and written into `sync.json`'s `mic_segments` / `webcam_segments` / `display_switches`.

### Used by

- `src-tauri/src/session/record/recorder.rs` - created in `start_take`, read in `pause_take` / `resume_take`.
- `src-tauri/src/session/record/recorder_stop.rs` - consumed in `stop_take`.

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
- `stopping: AtomicBool` - `true` while `stop_blocking` is tearing a session down. *Why needed at all:* the teardown runs OUTSIDE the lock (it joins threads and finalizes a possibly multi-GB MP4 - holding the recorder lock across that would block Pause/Resume on the main thread), and `Running` has already been taken by then, so `inner.is_some()` alone would wave a concurrent `start_recording` straight through. The Windows pointer adapter's hook sink is a process-global (`platform::windows::input::pointer::SINK`) that a start overwrites unconditionally, so that interleave writes the finished take's `events.json` from the NEW empty collector and leaves the new take's hook with no sink at all - zero mouse events, so no cursor, no click FX and no auto-zoom for its whole duration. *Why an atomic rather than a second mutex:* it is only ever read and written while holding `inner`'s lock, so it is effectively part of that guarded state and cannot introduce a lock-order cycle.

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

## TakeSpec

```rust
pub struct TakeSpec {
    pub base: PathBuf,
    pub project_name: String,
    pub mic_id: Option<String>,
    pub target_id: Option<String>,
    pub system_audio: bool,
    pub game_mode: bool,
}
```

Everything a caller asks a take FOR, as one value.

- `base: PathBuf` - the folder takes are created under. *Why a field and not `dirs_next::video_dir()` inside:* it is the one thing `start_take` would otherwise reach into the machine for, and a test that wrote real takes into the owner's Videos folder is not a test anyone would run twice. `start_recording` fills it with `video_dir()/TCursor`, falling back to `temp_dir` so recording never fails for lack of a video directory; `platform/mock/cycle_tests.rs` fills it with a temp directory it deletes.
- `project_name: String` - subfolder name under `base`. *Why caller-supplied:* the frontend generates a timestamp string so the name is predictable and unique without coordination.
- `mic_id: Option<String>` - cpal device name, or `None` to skip mic. *Why Option:* mic is optional; `spawn_mic_thread` converts `None` to an immediate `None` return.
- `target_id: Option<String>` - which display/window to capture, or `None` for the primary one. Parsed once into a `ports::capture::TargetId` at the port boundary, which is what stopped four hand-written parse sites from disagreeing about what `display:N` meant.
- `system_audio: bool` - whether to capture loopback audio. *Why bool not Option:* simple on/off; no device selection is needed for loopback.
- `game_mode: bool` - the compatibility toggle, `CaptureRequest::prefer_compatibility` at the port. On Windows it forces the legacy ffmpeg pipeline; an adapter with one pipeline ignores it.

*Why a struct and not six more arguments.* `start_take` would otherwise take ten, and a bare `(String, Option<String>, Option<String>, bool, bool)` is exactly the shape where two arguments of the same type get swapped silently.

## start_take

```rust
pub fn start_take(spec: TakeSpec, hooks: TakeHooks, platform: &Platform, recorder: &Recorder) -> Result<String, String>
```

Creates the project folder, starts all three input streams and the audio threads, starts the capture, stores the live session in `Recorder`, and returns the project folder path. Names no Tauri type: everything it reports goes through `hooks`, everything it touches on the OS goes through `platform`, and where it writes comes from `spec`.

### Inputs

- `spec: TakeSpec` - what to record and where, above.
- `hooks: TakeHooks` - the four callbacks a take reports through (`record-warning`, `record-ended-early`, and the two level feeds). *Why by value:* the take consumes them; the mic thread, the system thread and the capture each end up owning one.
- `platform: &Platform` - the adapter bundle. Used for the refresh rate, the three input streams, the loopback device and the capture start - which is all five of the OS-facing things a take start does.
- `recorder: &Recorder` - the singleton state this take is installed into.

### Returns

`Ok(String)` (the project folder) on success. `Err(String)` for: already recording (including the tail of a stop still in progress - see `Recorder::stopping`), folder creation failure, or a capture start failure. On any `CapturePort::start` error the mic/system-audio threads spawned earlier in this call are stopped and joined first (see step 6), so a failed start never leaks an open audio device or a running thread. The returned folder lets the HUD stream the webcam into it during recording.

An audio input that fails to OPEN is not an error here - the take is still worth having - so it arrives as a `record-warning` instead; see `recorder_threads::audio_warning`.

### Implementation

1. Lock `recorder.inner`. Return `Err("already recording")` if `Some` **or** if `stopping` is set. *Why check before any I/O:* prevents double-start from a fast frontend double-click with no wasted setup, and keeps a start out of the window where a previous stop is still detaching the global input hooks.
2. Create `ProjectPaths` under `spec.base`, call `ensure()`.
3. Snapshot settings via `settings::store::load` and write to `paths.settings()`. *Why snapshot at start:* the user might change settings mid-session; the export always uses the settings active at record time, ensuring reproducibility.
4. `platform.system.primary_refresh_hz()`, capped to 60. *Why cap:* 60fps is the practical ceiling for current targets, and a 144 Hz display should not cost CPU for frames that are thrown away. The clamp is a recording policy and stays here, at the call site, not in the port.
5. Create the `paused_totals: Arc<PauseTotals>` ledger. Build the three input streams through `platform.input` - `pointer(8, ledger)`, `hotkeys(arming_from_settings(&snap.hotkeys), ledger)`, `cursor_shapes(ledger)` - each given a clone of the SAME ledger so their stamp sites pause-adjust against one reading. Then start `spawn_mic_thread` and `spawn_system_thread`, each given the warning hook and its own level hook (`"mic"` / `"system"`), which is what feeds the HUD's live wave meter: one `audio-level` event per open source every ~50ms, straight from the capture that is writing the WAV, so a meter can only ever move for audio this take is really recording. The system thread is spawned only when `spec.system_audio` is set, and is handed `platform.audio.loopback_device()` rather than opening a device itself - `None` from the port reaches `SystemAudio::loopback` as the same `"no default output device"` it used to raise for itself. *Why the inputs come before the video pipeline:* building the encoder (first ffmpeg probe / Media Foundation setup) can take a moment. Starting inputs first ensures they capture from t=0 and do not miss that startup gap.
6. Build a `CaptureRequest` (the same `paused_totals`, the take's `stop` flag and the `record-ended-early` hook) and call `platform.capture.start` - the slow part - wrapped in a `match` (not `?`). It hands back the boxed sink and a `CaptureGeometry`. `with_cursor` is hardcoded `false` - the OS cursor is NEVER baked into the pixels for any style, because it is captured as its own layer instead (`events::track::cursorlayer`); that is what makes the cursor style an editor decision rather than a record-time one. *Why the ledger goes to the capture too:* that is the C1 fix - the encoded video's own PTS is compressed by the same ledger the input streams subtract, so `video.mp4` and `sync.json` share one clock. *Why `match` not `?`:* on `Err`, `stop` is stored `true` (`SeqCst`) and `mic_thread`/`system_thread` are joined before returning the error - otherwise those threads' 50ms poll loops would never see a stop signal (it is only ever set from `Running`, which is never constructed on this path) and would run forever, leaking the thread and leaving the mic/loopback device open with its WAV never finalized. The three input streams need no equivalent handling: they are plain locals at this point, so returning early drops them, and their `Drop` impls already stop the underlying hooks.
7. Build `ScreenInfo` from the `CaptureGeometry`, stamp `started_unix_ms`, and store everything in `Running`; assign to `*guard`.

### Behaviors

Pinned end to end by `platform/mock/cycle_tests.rs`, which starts, pauses, resumes and stops a take against `platform::mock` with no OS in the room: that a second start is refused while one is running, that the ports are driven in the order pointer / hotkeys / cursor shapes / capture, that `target_id: None` reaches the capture port as `TargetId::Primary`, and that the folder really holds `events.json`, `sync.json` and `project.tcursor` afterwards.

## pause_take

```rust
pub fn pause_take(recorder: &Recorder) -> Result<(), String>
```

Stamps `paused_totals` with the pause instant, then sets the shared `paused` flag to `true` with `SeqCst` ordering.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Call `r.paused_totals.pause(r.clock.now_ms())`. *Why before the flag flip, under the same lock:* the ledger and the flag must agree on the exact pause instant. Since C1/H2 this single reading is what BOTH the input streams and the video's own PTS are compressed by, so the whole recording removes exactly the same span. It is also why no port has a `pause()` method: one would move this stamp off this lock and let the streams disagree about a pause.
3. Store `true` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* ensures the video, mic, and system-audio threads observe the flag before the next iteration of their respective sleep loops (2-50ms latency is acceptable; SeqCst removes any reorder uncertainty).

## resume_take

```rust
pub fn resume_take(recorder: &Recorder) -> Result<(), String>
```

Closes the in-progress span on `paused_totals`, then clears the shared `paused` flag, resuming all capture threads.

### Returns

`Ok(())` if recording is active. `Err("not recording")` if `inner` is `None`.

### Implementation

1. Lock `recorder.inner`. Return error if `None`.
2. Call `r.paused_totals.resume(r.clock.now_ms())`. *Why:* folds the just-ended pause span into the ledger's accumulated total before any input stream - or either capture path - reads `elapsed_paused` again.
3. Store `false` into `r.paused` with `Ordering::SeqCst`. *Why SeqCst:* same reasoning as `pause_take`; the clearing write must be globally visible before the threads' next reads.

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
    platform: tauri::State<'_, Arc<Platform>>,
    app: tauri::AppHandle,
) -> Result<String, String>
```

The Tauri wrapper over `start_take`: resolves `base` from `dirs_next::video_dir()`, builds the `TakeHooks` from the `AppHandle`, and on success swaps the main window's icon to the REC-lit variant (`shell::brand_icon::set_recording`). The icon is brand flair only - never fails the command - and it runs after `start_take` returns `Ok`, so it only ever fires on an actual successful start.

- `platform: tauri::State<'_, Arc<Platform>>` - the process-wide adapter bundle, built once in `lib.rs`'s `setup`. An injected parameter like `recorder`, so the JS call (`invoke("start_recording", {...})`) is unchanged.
- `app: tauri::AppHandle` - source of the four hooks (see `emit::TakeHooks::from_app`) and of the window whose icon is swapped.

## pause_recording

```rust
#[tauri::command]
pub fn pause_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

The Tauri wrapper over `pause_take`.

## resume_recording

```rust
#[tauri::command]
pub fn resume_recording(recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

The Tauri wrapper over `resume_take`.
