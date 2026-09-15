# src-tauri/src/platform/windows/input/pointer.rs

Windows low-level mouse hook that captures global cursor movements and button clicks on a dedicated message-loop thread and funnels them into an `EventCollector` via a process-global `Mutex`. Dropping `Win32Pointer` without calling `stop` is safe - the `Drop` impl posts `WM_QUIT` and joins the thread.

This is the `PointerPort` implementation for Windows (the trait impl itself is in `mod.rs`). `events::track::tracker::MouseTracker` is an alias for `Win32Pointer` so the recorder's call sites keep their spelling until the composition root is wired.

*Why the global lives here.* `static SINK` is private to this file, reachable only through the handle. It is contained, not removed: `SetWindowsHookExW` takes a bare `unsafe extern "system" fn` with no user pointer, so the callback has no way to reach a captured collector, and the alternatives (a thread-local, a channel drain, a handle registry) each either need a channel anyway for `set_remap` and `stop`, or move the timestamp off the sink's own clock and break the one-stamp-site invariant `push_polled_move` depends on.

## Sink

```rust
struct Sink { start: Instant, collector: EventCollector, ledger: Arc<PauseTotals>, remap: Option<Remap> }
```

The process-global the hook reaches the recording through, held in `static SINK: Mutex<Option<Sink>>`. *Why a global:* `hook_proc` is an `unsafe extern "system"` function that cannot capture a closure or hold a reference, so there is no other way for it to find the collector.

- `start` / `ledger` - the two halves of a sample's timestamp: raw elapsed ms since the hook was installed, pause-compressed by the recorder's shared `PauseTotals`.
- `remap: Option<Remap>` - `None` for the whole of an ordinary take, and `Some` only while `session::record::switch_display` has moved the capture to another monitor. Written by `Win32Pointer::set_remap` under the same mutex the hook reads it under.

## remapped

```rust
fn remapped(remap: Option<&Remap>, x: i32, y: i32) -> (i32, i32)
```

The point one hook sample contributes: the raw desktop coordinate, or where it lands on the take's own display once a mid-take display switch has installed a `Remap` (`events/remap.rs`). A named function rather than an inline `match` so the mapping can be exercised at the stamp site without a live Windows hook, which is how the tests below reach it.

## stamp_push

```rust
fn stamp_push(sink: &mut Sink, kind: EventKind, x: i32, y: i32, button: Option<Button>)
```

The ONE stamp site. Every sample, whether the hook saw it or a poll did, takes its time off the sink's own clock while the sink's lock is held (so push order is time order across the two threads), goes through the pause ledger (`PauseTotals::stamp`) and the display remap (`remapped`), and only then reaches the collector. `hook_proc` and `push_polled_move` both call it, so the two sources can never drift apart in how they stamp.

## push_polled_move

```rust
pub(crate) fn push_polled_move(x: i32, y: i32)
```

A pointer position seen by a POLL rather than by the hook, pushed as a `Move`. `cursor.rs` reads `GetCursorInfo` every 16 ms for the cursor shape and hands the `ptScreenPos` it got for free to this function.

*Why it exists (2026-09-15).* `WH_MOUSE_LL` only runs for motion that goes through the input stack. A cursor warped with `SetCursorPos` (automation and macro tools, remote desktop, eye trackers, some pen and VM drivers) moves on screen without a single hook callback, while clicks sent with `mouse_event`/`SendInput` still arrive. A take made that way held its four clicks and not one move, so the editor drew the cursor parked at the previous click while the site under it showed the pointer somewhere else entirely. The poll is the position source of truth that works however the cursor got there; the hook stays the high-rate source and the click source.

*Cost.* `EventCollector::push` drops a `Move` whose coordinates equal the last one, so with a real mouse the poll adds nothing the hook did not already deliver, and the 8 ms move throttle still applies. The poll runs at the shape tracker's 60 Hz, which is above the 30 to 60 fps the export samples the path at.

*Safety.* A silent no-op before the hook thread has filled the sink and after `stop` has taken it, and poison-tolerant like `hook_proc`: the poller can never panic the recording path. Coordinates are physical desktop pixels, the same space the hook reports, because the process is per-monitor DPI aware.

### Used by

- `src-tauri/src/platform/windows/input/cursor.rs` - once per `GetCursorInfo` poll.

## Win32Pointer

```rust
pub struct Win32Pointer {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}
```

RAII handle for the mouse hook thread. `thread` is `Option` so `stop` and `Drop` can both `take` it without a double-join.

- `thread` - *join handle for the message-loop thread; `None` after `stop` or `Drop` has joined it.*
- `thread_id` - *Windows thread id of the hook thread, needed by `PostThreadMessageW` to post `WM_QUIT` from a different thread.*

### Used by

- `src-tauri/src/platform/windows/input/mod.rs` - the `PointerPort` impl and `Win32Input::pointer`
- `src-tauri/src/session/record/recorder.rs` - started at recording begin, under the alias `MouseTracker`; `stop` called at recording end to collect events
- `src-tauri/src/session/record/recorder_threads.rs` - orchestrates tracker lifecycle alongside `CursorTypeTracker`

## Win32Pointer::start

```rust
pub fn start(move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Self
```

Spawns the hook thread and returns a handle. Returns immediately after the thread has installed the hook.

### Inputs

- `move_min_interval_ms: u32` - forwarded to `EventCollector::new` inside the hook thread. *Why passed here rather than read from settings inside the thread:* the caller already has the resolved config; threading it in avoids a re-read from disk on the hook thread.*
- `ledger: Arc<PauseTotals>` - the recorder's exact-span paused-time ledger. *Why:* `hook_proc` runs on the hook thread and has no other way to reach the recorder's pause state; every event's raw elapsed-ms reading is pause-adjusted via `ledger.stamp` before being pushed to the collector, so recorded events land in the same pause-compressed timeline as video/audio.

### Returns

`Win32Pointer` with `thread_id` set to the OS thread id of the hook thread. If `spawn` fails, `thread` is `None` and `thread_id` is `0`; `stop` degrades gracefully to an empty event list.

### Implementation

1. Create an `mpsc::channel` to receive the hook thread's OS thread id.
2. Spawn a named thread `"mouse-hook"`.
3. Inside the thread: call `run(move_min_interval_ms, ledger, callback)`:
   a. Initialize `SINK` with a new `Sink { start: Instant::now(), collector: EventCollector::new(...), ledger }`.
   b. Get and send the OS thread id via `GetCurrentThreadId` + the channel callback.
   c. Install the low-level mouse hook via `SetWindowsHookExW(WH_MOUSE_LL, hook_proc, ...)`. *Why `WH_MOUSE_LL`:* a low-level hook receives global mouse events regardless of which window has focus, which is required to capture clicks outside the TCursor window during recording.*
   d. Run `GetMessageW` in a loop until `WM_QUIT`. Every OS mouse event triggers `hook_proc`.
   e. On `WM_QUIT`: unhook via `UnhookWindowsHookEx` and return.
4. `hook_proc`: maps the Windows message (`WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, etc.) to `(EventKind, Option<Button>)`, acquires `SINK`, computes the raw elapsed ms via `sink.start.elapsed()`, pause-adjusts it via `sink.ledger.stamp(raw)`, passes `info.pt` through `remapped(sink.remap.as_ref(), ...)`, and calls `collector.push` with the adjusted time and the mapped point. Always calls `CallNextHookEx` to not break the global hook chain.
5. The whole file is Windows-only by its place in the tree: `platform/mod.rs` gates `pub mod windows;`, so there are no inner `#[cfg]` arms left to keep in step.

### Behaviors

- `paused_span_is_subtracted_before_reaching_the_collector` - mirrors the `hook_proc` stamp site (raw elapsed ms -> `ledger.stamp` -> `collector.push`) without a real Windows hook: with the ledger paused 1000..3000, a raw reading of 3100 reaches the collector as `t = 1100`.
- `a_switched_display_is_remapped_before_reaching_the_collector` - the other half of the same stamp site: with a `Remap` for a 1280x800 second monitor at `(1920, 0)` into a 1920x1080 take, the raw hook point `(2560, 400)` reaches the collector as `(960, 540)`; with the remap cleared it reaches the collector unchanged.
- `a_polled_position_lands_as_a_move_through_the_hook_stamp_site` - the poll's path, end to end, against the real `SINK`: a push before the sink is filled is a no-op, two identical polls collapse to one `Move`, the point takes the display remap the hook's points take, and a push after `stop` has taken the sink leaves it taken. This is the test that pins the one-stamp-site invariant, so it holds the global for its whole body.

## Win32Pointer::set_remap

```rust
pub fn set_remap(&self, remap: Option<Remap>)
```

Installs (or clears) the display remap every later sample is mapped through. Called by `session::record::switch_display` under the recorder lock, after the capture has actually moved: until then the pixels are still the old display's and the raw coordinates are the correct ones.

`None` restores raw desktop coordinates, which is what a switch BACK to the take's own display wants - clearing the mapping rather than installing an identity one keeps the ordinary case free on the hook's hot path. A no-op before `start`'s thread has filled the sink, and poison-tolerant like every other lock on the recording path: losing the mapping must not turn a source switch into a panicked command.

*Why `&self` and not `&mut self`:* the state lives in the process-global `SINK`, not in the handle, and `Running` holds the tracker behind a shared reference for the life of the take.

## Win32Pointer::stop

```rust
pub fn stop(mut self) -> Vec<MouseEvent>
```

Signals the hook thread to exit, joins it, and returns the collected events.

### Inputs

- `self` - consumed. *Why consuming:* prevents calling `stop` twice or mixing `stop` with `Drop` cleanup.*

### Returns

`Vec<MouseEvent>` - the full, filtered event list from `EventCollector::take`. Empty if the thread was never started or if the hook produced no events.

### Implementation

1. Post `WM_QUIT` to the hook thread via `PostThreadMessageW(thread_id, WM_QUIT, ...)`. *Why post rather than a flag:* the hook thread blocks in `GetMessageW`; posting `WM_QUIT` is the canonical way to unblock it.*
2. `take` the thread handle and join it (discarding the `()` return).
3. Lock `SINK`, take the `Sink`, call `collector.take()`. *Why the global `Mutex<Option<Sink>>`:* `hook_proc` is a `unsafe extern "system"` function that cannot capture a closure or hold a reference, so the only way to reach the collector from the hook is via a process-global.*
