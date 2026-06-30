# src-tauri/src/events/cursortracker.rs

Background thread that polls the Windows cursor shape at ~60 Hz and records a timestamped log of shape-change events. Enhanced screen capture hides the OS cursor from the video stream, so the shape must be sampled live and stored for later rendering by the cursor compositor.

## CursorTypeTracker

```rust
pub struct CursorTypeTracker {
    thread: Option<JoinHandle<Vec<(u32, CursorType)>>>,
    stop: Arc<AtomicBool>,
}
```

RAII handle for the polling thread. Dropping without calling `stop` is safe - the `Drop` impl signals and joins the thread to prevent leaks.

- `thread` - *join handle for the polling thread; `Option` so `stop` and `Drop` can both `take` it without a double-join.*
- `stop` - *shared flag; the polling loop checks it on every iteration (~16 ms) so shutdown latency is at most one poll interval.*

### Used by

- `src-tauri/src/session/recorder.rs` - starts the tracker at recording begin; calls `stop` when recording ends to collect the sample log
- `src-tauri/src/session/recorder_threads.rs` - orchestrates the tracker lifecycle alongside `MouseTracker` and the frame grabber

## CursorTypeTracker::start

```rust
pub fn start() -> Self
```

Spawns the polling thread and returns a handle to it. Returns immediately; polling runs concurrently.

### Inputs

None. *Why no config parameter:* the poll interval (16 ms, ~60 Hz) is a fixed hardware-aligned constant; the classification table is built once inside the thread from the OS standard cursor set.*

### Returns

`CursorTypeTracker` with the thread running. If `thread::spawn` fails (extremely rare, OS resource exhaustion), `thread` is `None` and `stop` returns an empty vec.

### Implementation

1. Create `Arc<AtomicBool>` stop flag.
2. Spawn a named thread `"cursor-type"`.
3. Inside the thread (Windows only): call `imp::run(stop)`.
   - `classify_table()` calls `LoadCursorW` once for each standard IDC constant to build a `(HCURSOR, CursorType)` lookup table. *Why build at thread start rather than compile time:* `HCURSOR` values are runtime handles, not compile-time constants.*
   - Poll loop (every 16 ms while `!stop`): call `GetCursorInfo`. Match `info.hCursor` against the table. On a recognized shape change, append `(elapsed_ms, ty)`. On an unrecognized (custom app) cursor, keep the last known type rather than guessing. If the very first sample is custom, seed with `Arrow` so `type_at` always has a base entry.
4. On non-Windows builds, the thread body is a no-op that returns `Vec::new()`.

## CursorTypeTracker::stop

```rust
pub fn stop(mut self) -> Vec<(u32, CursorType)>
```

Signals the polling thread to exit, joins it, and returns the collected shape-change log.

### Inputs

- `self` - consumed. *Why consuming:* enforces that `stop` is called at most once, preventing double-signal or attempts to collect twice.*

### Returns

`Vec<(u32, CursorType)>` - one entry per shape change (not per poll tick), in ascending time. Passed to `CursorTrack` for persistence and later playback via `CursorTrack::type_at`.

### Implementation

1. Set `stop` flag to `true` via `Relaxed` store. *Why Relaxed:* the flag guards no shared data other than itself; the subsequent `join` provides the necessary ordering.*
2. `take` the thread handle and join it. `unwrap_or_default` on the join result treats a panicked thread as empty (rare; panics in the polling loop would be an OS API failure).
