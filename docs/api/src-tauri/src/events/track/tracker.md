# src-tauri/src/events/track/tracker.rs

Windows low-level mouse hook that captures global cursor movements and button clicks on a dedicated message-loop thread and funnels them into an `EventCollector` via a process-global `Mutex`. Dropping `MouseTracker` without calling `stop` is safe - the `Drop` impl posts `WM_QUIT` and joins the thread.

## Sink

```rust
struct Sink { start: Instant, collector: EventCollector, ledger: Arc<PauseTotals>, remap: Option<Remap> }
```

The process-global the hook reaches the recording through, held in `static SINK: Mutex<Option<Sink>>`. *Why a global:* `hook_proc` is an `unsafe extern "system"` function that cannot capture a closure or hold a reference, so there is no other way for it to find the collector.

- `start` / `ledger` - the two halves of a sample's timestamp: raw elapsed ms since the hook was installed, pause-compressed by the recorder's shared `PauseTotals`.
- `remap: Option<Remap>` - `None` for the whole of an ordinary take, and `Some` only while `session::record::switch_display` has moved the capture to another monitor. Written by `MouseTracker::set_remap` under the same mutex the hook reads it under.

## remapped

```rust
fn remapped(remap: Option<&Remap>, x: i32, y: i32) -> (i32, i32)
```

The point one hook sample contributes: the raw desktop coordinate, or where it lands on the take's own display once a mid-take display switch has installed a `Remap` (`events/remap.rs`). A named function rather than an inline `match` so the mapping can be exercised at the stamp site without a live Windows hook, which is how the tests below reach it.

## MouseTracker

```rust
pub struct MouseTracker {
    thread: Option<JoinHandle<()>>,
    thread_id: u32,
}
```

RAII handle for the mouse hook thread. `thread` is `Option` so `stop` and `Drop` can both `take` it without a double-join.

- `thread` - *join handle for the message-loop thread; `None` after `stop` or `Drop` has joined it.*
- `thread_id` - *Windows thread id of the hook thread, needed by `PostThreadMessageW` to post `WM_QUIT` from a different thread.*

### Used by

- `src-tauri/src/session/record/recorder.rs` - started at recording begin; `stop` called at recording end to collect events
- `src-tauri/src/session/record/recorder_threads.rs` - orchestrates tracker lifecycle alongside `CursorTypeTracker`

## MouseTracker::start

```rust
pub fn start(move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Self
```

Spawns the hook thread and returns a handle. Returns immediately after the thread has installed the hook.

### Inputs

- `move_min_interval_ms: u32` - forwarded to `EventCollector::new` inside the hook thread. *Why passed here rather than read from settings inside the thread:* the caller already has the resolved config; threading it in avoids a re-read from disk on the hook thread.*
- `ledger: Arc<PauseTotals>` - the recorder's exact-span paused-time ledger. *Why:* `hook_proc` runs on the hook thread and has no other way to reach the recorder's pause state; every event's raw elapsed-ms reading is pause-adjusted via `ledger.stamp` before being pushed to the collector, so recorded events land in the same pause-compressed timeline as video/audio.

### Returns

`MouseTracker` with `thread_id` set to the OS thread id of the hook thread. If `spawn` fails, `thread` is `None` and `thread_id` is `0`; `stop` degrades gracefully to an empty event list.

### Implementation

1. Create an `mpsc::channel` to receive the hook thread's OS thread id.
2. Spawn a named thread `"mouse-hook"`.
3. Inside the thread (Windows): call `imp::run(move_min_interval_ms, ledger, callback)`:
   a. Initialize `SINK` with a new `Sink { start: Instant::now(), collector: EventCollector::new(...), ledger }`.
   b. Get and send the OS thread id via `GetCurrentThreadId` + the channel callback.
   c. Install the low-level mouse hook via `SetWindowsHookExW(WH_MOUSE_LL, hook_proc, ...)`. *Why `WH_MOUSE_LL`:* a low-level hook receives global mouse events regardless of which window has focus, which is required to capture clicks outside the TCursor window during recording.*
   d. Run `GetMessageW` in a loop until `WM_QUIT`. Every OS mouse event triggers `hook_proc`.
   e. On `WM_QUIT`: unhook via `UnhookWindowsHookEx` and return.
4. `hook_proc`: maps the Windows message (`WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, etc.) to `(EventKind, Option<Button>)`, acquires `SINK`, computes the raw elapsed ms via `sink.start.elapsed()`, pause-adjusts it via `sink.ledger.stamp(raw)`, passes `info.pt` through `remapped(sink.remap.as_ref(), ...)`, and calls `collector.push` with the adjusted time and the mapped point. Always calls `CallNextHookEx` to not break the global hook chain.
5. On non-Windows: thread body is a no-op; `thread_id = 0`.

### Behaviors

- `paused_span_is_subtracted_before_reaching_the_collector` - mirrors the `hook_proc` stamp site (raw elapsed ms -> `ledger.stamp` -> `collector.push`) without a real Windows hook: with the ledger paused 1000..3000, a raw reading of 3100 reaches the collector as `t = 1100`.
- `a_switched_display_is_remapped_before_reaching_the_collector` - the other half of the same stamp site: with a `Remap` for a 1280x800 second monitor at `(1920, 0)` into a 1920x1080 take, the raw hook point `(2560, 400)` reaches the collector as `(960, 540)`; with the remap cleared it reaches the collector unchanged.

## MouseTracker::set_remap

```rust
pub fn set_remap(&self, remap: Option<Remap>)
```

Installs (or clears) the display remap every later sample is mapped through. Called by `session::record::switch_display` under the recorder lock, after the capture has actually moved: until then the pixels are still the old display's and the raw coordinates are the correct ones.

`None` restores raw desktop coordinates, which is what a switch BACK to the take's own display wants - clearing the mapping rather than installing an identity one keeps the ordinary case free on the hook's hot path. A no-op before `start`'s thread has filled the sink, and poison-tolerant like every other lock on the recording path: losing the mapping must not turn a source switch into a panicked command.

*Why `&self` and not `&mut self`:* the state lives in the process-global `SINK`, not in the handle, and `Running` holds the tracker behind a shared reference for the life of the take.

## MouseTracker::stop

```rust
pub fn stop(mut self) -> Vec<crate::events::model::MouseEvent>
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
