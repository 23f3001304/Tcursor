# src-tauri/src/platform/windows/input/cursor.rs

Background thread that polls the Windows cursor at ~60 Hz. It records two things: a timestamped log of cursor-TYPE changes (which sprite the Enhanced style replays), and the real cursor BITMAPS as their own layer (what the System style composites). Screen capture now excludes the OS cursor for every style, so both the shape and the pixels only exist if they are sampled live here.

It therefore runs on every take, not just Enhanced ones - a Hidden or Enhanced recording switched to System in the editor needs the layer just as much as a System one does.

This is the `CursorShapePort` implementation for Windows (the trait impl itself is in `mod.rs`). `events::track::cursortracker::CursorTypeTracker` is an alias for `Win32CursorShapes`, and `CursorSamples` stays in that module because both halves of it are portable: a macOS or Linux adapter fills the same tuple.

## Win32CursorShapes

```rust
pub struct Win32CursorShapes {
    thread: Option<JoinHandle<CursorSamples>>,
    stop: Arc<AtomicBool>,
}
```

RAII handle for the polling thread. Dropping without calling `stop` is safe - the `Drop` impl signals and joins the thread to prevent leaks.

- `thread` - *join handle for the polling thread; `Option` so `stop` and `Drop` can both `take` it without a double-join.*
- `stop` - *shared flag; the polling loop checks it on every iteration (~16 ms) so shutdown latency is at most one poll interval.*

### Used by

- `src-tauri/src/platform/windows/input/mod.rs` - the `CursorShapePort` impl and `Win32Input::cursor_shapes`
- `src-tauri/src/session/record/recorder.rs` - starts the tracker at recording begin, under the alias `CursorTypeTracker`; calls `stop` when recording ends to collect the sample log
- `src-tauri/src/session/record/recorder_threads.rs` - orchestrates the tracker lifecycle alongside `MouseTracker` and the frame grabber

## Win32CursorShapes::start

```rust
pub fn start(ledger: Arc<PauseTotals>) -> Self
```

Spawns the polling thread and returns a handle to it. Returns immediately; polling runs concurrently.

### Inputs

- `ledger: Arc<PauseTotals>` - the recorder's exact-span paused-time ledger. *Why:* every shape-change sample's raw elapsed-ms reading is pause-adjusted via `ledger.stamp` before being recorded, so the cursor-type timeline lands in the same pause-compressed timeline as video/audio. The poll interval itself (16 ms, ~60 Hz) is still a fixed hardware-aligned constant; the classification table is built once inside the thread from the OS standard cursor set.

### Returns

`Win32CursorShapes` with the thread running. If `thread::spawn` fails (extremely rare, OS resource exhaustion), `thread` is `None` and `stop` returns empty samples and an empty layer.

### Implementation

1. Create `Arc<AtomicBool>` stop flag.
2. Spawn a named thread `"cursor-type"`.
3. Inside the thread: call `run(stop, ledger)`.
   - `classify_table()` calls `LoadCursorW` once for each standard IDC constant to build a `(HCURSOR, CursorType)` lookup table. *Why build at thread start rather than compile time:* `HCURSOR` values are runtime handles, not compile-time constants.*
   - Poll loop (every 16 ms while `!stop`): call `GetCursorInfo`, then take ONE `ledger.stamp` reading for the tick. *Why one:* both logs are appended from it, so a shape change and the bitmap change that goes with it can never land on different milliseconds.
   - **Type log.** Match `info.hCursor` against the table. On a recognized shape change, append `(t, ty)`. On an unrecognized (custom app) cursor, keep the last known type rather than guessing. If the very first sample is custom, seed with `Arrow` so `type_at` always has a base entry.
   - **Bitmap layer** (`track_shape`). A handle not seen before is captured once via `bitmap::capture` and added to the `CursorLayerBuilder`; the handle -> id map also remembers FAILURES (as `None`) so an uncapturable cursor is not retried 60 times a second, and the cap (`is_full`) is checked before any GDI work. When the resolved id differs from the last one, `(t, id)` is appended. A poll whose handle has no bitmap leaves the previous id in force, exactly as an unrecognized handle leaves the previous type in force. There is no `Arrow`-style fallback here on purpose: a custom app cursor is precisely the case the type track cannot describe, and the one the captured layer gets right.
4. The whole file is Windows-only by its place in the tree: `platform/mod.rs` gates `pub mod windows;`, so the thread body has no `#[cfg]` arms of its own.

## Win32CursorShapes::stop

```rust
pub fn stop(mut self) -> CursorSamples
```

Signals the polling thread to exit, joins it, and returns the collected shape-change log and cursor layer.

### Inputs

- `self` - consumed. *Why consuming:* enforces that `stop` is called at most once, preventing double-signal or attempts to collect twice.*

### Returns

`CursorSamples` - the `Vec<(u32, CursorType)>` shape log (one entry per shape change, not per poll tick, in ascending time), passed to `CursorTrack` for persistence and later playback via `CursorTrack::type_at`; and the `CursorLayerBuilder`, passed to `CursorLayerBuilder::save`.

### Implementation

1. Set `stop` flag to `true` via `Relaxed` store. *Why Relaxed:* the flag guards no shared data other than itself; the subsequent `join` provides the necessary ordering.*
2. `take` the thread handle and join it. `unwrap_or_default` on the join result treats a panicked thread as empty (rare; panics in the polling loop would be an OS API failure).

### The position goes to the mouse track too (2026-09-15)

Every successful `GetCursorInfo` poll hands its `ptScreenPos` to `pointer::push_polled_move` before it stamps the shape. The `WH_MOUSE_LL` hook in `pointer.rs` never sees motion that bypasses the input stack (a `SetCursorPos` warp from automation, remote desktop, an eye tracker, some pen and VM drivers), and a take made that way used to hold its clicks and not one move. The poll is what makes the position track follow the cursor however it got where it is; `EventCollector` drops the sample when the hook already delivered the same point, so a real mouse costs nothing. See `pointer.md`.
