# src-tauri/src/session/record/recorder_threads.rs

Thread-spawning helpers and input-persistence logic factored out of `recorder.rs` to keep that file under the 200-line cap. Every function here is called from a single site in `recorder.rs`. The two `spawn_*` functions each create `!Send` audio handles on a dedicated thread; `save_inputs` runs synchronously on the main stop path before joining the video thread.

## save_inputs

```rust
pub fn save_inputs(
    mouse: Option<MouseTracker>,
    keyboard: Option<KeyboardTracker>,
    cursor: Option<CursorTypeTracker>,
    events_path: &Path,
    actions_path: &Path,
    typing_path: &Path,
    cursor_path: &Path,
    screen: ScreenInfo,
    started_unix_ms: u64,
)
```

Stops each active input tracker and writes its data to disk. Called synchronously on the `stop_recording` path, before joining the video thread, so input data is always flushed even if the video encoder fails.

### Inputs

- `mouse: Option<MouseTracker>` - `Some` when a mouse tracker was started; `None` skips this track. *Why Option:* mouse tracking may be absent in test or minimal builds.
- `keyboard: Option<KeyboardTracker>` - `Some` when a keyboard tracker was started. *Why Option:* stops and splits into `(actions, typing)` in one call; either or both tracks may be absent.
- `cursor: Option<CursorTypeTracker>` - `Some` only when `CursorStyle::Enhanced` was active. *Why Option:* Enhanced mode is the only mode that draws a synthetic cursor requiring shape data; System and Hidden modes skip this tracker entirely.
- `events_path: &Path` - destination for `events.json`. *Why Path not String:* the serialization helpers in `EventLog` take `&Path`; avoids re-allocation.
- `actions_path / typing_path / cursor_path: &Path` - destinations for `actions.json`, `typing.json`, `cursor.json` respectively.
- `screen: ScreenInfo` - capture dimensions and origin embedded in `EventLog`. *Why passed here:* `EventLog` must describe the screen geometry used during recording so the exporter can map pixel coordinates correctly.
- `started_unix_ms: u64` - Unix epoch time (ms) of recording start, embedded in `EventLog`. *Why:* the exporter and future tools can correlate events to wall-clock time.

### Returns

`()`. Individual save errors are logged to `stderr` but do not propagate; a partial write failure does not abort the others.

### Implementation

1. If `mouse` is `Some`: call `tracker.stop()` to drain the event queue, construct `EventLog { started_unix_ms, screen, events }`, call `log.save(events_path)`. Log any error to stderr and continue.
2. If `keyboard` is `Some`: call `kb.stop()` which returns `(actions, typing)`. Save `ActionLog { actions }` to `actions_path` and `TypingLog { ms: typing }` to `typing_path`. Use `.ok()` on the typing save since typing data is best-effort. *Why stop returns both:* the keyboard tracker collects both action events and raw keystroke timestamps on one hook; a single stop call drains both queues atomically.
3. If `cursor` is `Some`: call `c.stop()` to get `samples`, save `CursorTrack { samples }` to `cursor_path`. Log error to stderr.

## save_session_files

```rust
pub fn save_session_files(folder: &str, frames: Vec<u64>, events_ms: u64,
    mic_ms: Option<u64>, system_ms: Option<u64>, screen: ScreenInfo)
```

Persists the three post-capture session files: `sync.json` (the real capture timeline the export rebuilds every clock from), the `.tcursor` project manifest, and the recents entry. Split out of `stop_recording` so `recorder.rs` stays under the line cap, alongside `save_inputs`.

### Inputs (what, and why it is needed)

- `folder: &str` - the project directory. *Why:* `sync.json` is written directly under it, and `ProjectPaths` is rebuilt from it for the manifest.
- `frames: Vec<u64>` - per-frame capture timestamps (ms) from `VideoSink::stop_and_collect`. *Why:* this is the recording's true, VFR timeline; export is fps-agnostic and rebuilds every clock from it.
- `events_ms: u64` - the input clock's epoch. *Why:* it is what maps event timestamps onto the video timeline (`output_shift`).
- `mic_ms` / `system_ms: Option<u64>` - each audio stream's first-sample capture time, or `None` when that stream was off. *Why:* they cancel device input latency when the export mixes audio. The caller maps a stored `0` to `None`.
- `screen: ScreenInfo` - the captured monitor's size/origin. *Why:* the manifest records the source dimensions.

### Returns

Nothing. All three writes are **best-effort**: `sync.json` and the manifest log to `eprintln!` on failure and `recents::touch` swallows its own errors. A failure here must never fail the recording - the folder is already a fully valid project without the manifest, just not open-project-able by dialog until the next write.

### Why `preprocessed: false`

The manifest is written with `preprocessed: false` regardless. The frontend calls `export::preview::preprocess::preprocess_project` right after `stop_recording` resolves (shown as the HUD's "Saving..." progress via `useRecordingFlow`) and that flips it once its pass finishes. This is NOT done as a detached background thread from the stop path anymore - that used to race the editor's mount (the proxy/thumbs/waveform transcode could still be running when the editor opened), which was exactly the "preview still takes a while to load" lag; awaiting it with progress in the caller fixes that.

## spawn_mic_thread

```rust
pub fn spawn_mic_thread(
    mic_id: Option<String>,
    mic_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
) -> Option<JoinHandle<()>>
```

Spawns a dedicated thread that owns and drives a `CpalMic` handle. Returns `None` immediately if `mic_id` is `None`.

### Inputs

- `mic_id: Option<String>` - cpal input device name. *Why Option:* `None` short-circuits the function before any thread is created; the caller stores the result directly as `mic_thread`.
- `mic_path: String` - destination for `mic.wav`. *Why String:* moved into the thread closure, where it is passed to `CpalMic::open` as a `&str`.
- `stop: Arc<AtomicBool>` - shared shutdown flag. *Why Arc:* the thread loop polls it at 50ms intervals; storing the clone inside the thread ensures the borrow never escapes.
- `paused: Arc<AtomicBool>` - pause flag passed to `CpalMic::open` so the mic stream can gate sample capture. *Why passed at open time:* cpal callbacks run on an OS audio thread; the `Arc` is the only safe channel to communicate pause state into the callback.
- `clock: Arc<dyn Clock>` - used inside `CpalMic::open` to stamp `started` with the first sample's capture time, cancelling device input latency. *Why Arc<dyn Clock>:* injectable for tests; `SystemClock` is the production instance.
- `started: Arc<AtomicU64>` - written by `CpalMic` at the first captured sample. *Why AtomicU64:* read back from the main thread in `stop_recording` without a lock; SeqCst is used on both sides.

### Returns

`Some(JoinHandle<()>)` on successful thread spawn; `None` if `mic_id` is `None`. A `spawn` failure also produces `None` via `.ok()` (uncommon; indicates OS thread exhaustion).

### Implementation

1. `let id = mic_id?` - short-circuits to `None` if mic is off.
2. Spawn thread named `"mic"`.
3. Inside the thread: open `CpalMic` with the device id, path, pause flag, started counter, and clock. Store the handle (or log the error and store `None`). *Why store `None` on error rather than abort:* a mic failure is non-fatal; recording continues without audio.
4. Loop sleeping 50ms until `stop` is set. *Why 50ms not 2ms:* audio is driven by the cpal callback, not this loop; the loop only needs to keep the handle alive and check for stop.
5. On stop: call `handle.stop()` to flush and close the wav file.

## spawn_system_thread

```rust
pub fn spawn_system_thread(
    enabled: bool,
    system_path: String,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    clock: Arc<dyn Clock>,
    started: Arc<AtomicU64>,
) -> Option<JoinHandle<()>>
```

Spawns a dedicated thread that owns and drives a `SystemAudio` loopback handle. Returns `None` immediately if `enabled` is false.

### Inputs

- `enabled: bool` - whether system-audio capture is active. *Why bool not Option:* unlike mic, there is no device selection; it is simply on or off.
- `system_path: String` - destination for `system.wav`. *Why String:* moved into the closure.
- `stop: Arc<AtomicBool>` - shared shutdown flag, same semantics as in `spawn_mic_thread`.
- `paused: Arc<AtomicBool>` - pause flag passed to `SystemAudio::loopback` so loopback samples are gated during pause.
- `clock: Arc<dyn Clock>` - forwarded into `SystemAudio::loopback`, which stamps `started` from inside the data callback at the first non-empty packet - mirroring the mic's in-callback pattern (see `spawn_mic_thread`) instead of stamping at stream-open. *Why not stamp here anymore:* stream-open (config negotiation + `stream.play()`) measurably precedes when samples actually start arriving; stamping in the callback removes that gap the same way the mic path removes its device latency.
- `started: Arc<AtomicU64>` - passed straight into `SystemAudio::loopback`, which owns the stamping. *Why SeqCst store:* read back on the main thread in `stop_recording`; must be globally visible before the `join` returns.

### Returns

`Some(JoinHandle<()>)` on successful thread spawn; `None` if `enabled` is false or spawn fails.

### Implementation

1. `if !enabled { return None; }`.
2. Spawn thread named `"system-audio"`.
3. Inside the thread: call `SystemAudio::loopback(&system_path, paused, started, clock)`, moving `started`/`clock` in directly - `loopback` itself stamps `started` at the first non-empty callback packet. On success, store the handle. On failure, log and store `None`. *Why store `None` on error:* same as mic - non-fatal; recording continues without system audio.
4. Loop sleeping 50ms until `stop` is set.
5. Call `handle.stop()` to flush and close the wav file. *Why `SystemAudioHandle` is `!Send`:* cpal streams contain platform handles that must be released on the same thread they were created on; owning the handle in the thread that opened it satisfies this invariant.
