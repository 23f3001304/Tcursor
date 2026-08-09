# src-tauri/src/capture/windows_capture.rs

Bridges the Windows Graphics Capture (WGC) API's callback-based frame delivery into the pull-based `FrameSource` interface via an internal mpsc channel. The key properties are: WGC runs its callback on its own thread (WGC thread); the encoder thread pulls frames via `next_frame` or `drain_latest`; and row padding is stripped inside the callback so consumers receive tightly-packed BGRA rows. Shutdown requires two coordinated steps - set the halt handle and call the stopper - to avoid leaking the WGC thread.

## WgcFrameSource

```rust
pub struct WgcFrameSource {
    rx: Receiver<Frame>,
    dims: (u32, u32),
    halt: Arc<AtomicBool>,
    stopper: Option<Box<dyn FnOnce() + Send>>,
    drops: Arc<AtomicU64>,
}
```

Implements `FrameSource` on top of a WGC capture session running on its own OS thread.

- `rx: Receiver<Frame>` - *Pull end of the bounded (`sync_channel(8)`) mpsc channel. `next_frame` blocks on `recv()`; `drain_latest` drains with `try_recv`. When the WGC thread exits and drops the `SyncSender`, `recv()` returns `Err`, causing `next_frame` to return `None` and the encoder loop to exit.*
- `dims: (u32, u32)` - *Primary monitor dimensions captured once at construction. Returned by `dimensions()` without waiting for a frame.*
- `halt: Arc<AtomicBool>` - *WGC halt handle. Setting it to `true` signals the capture session to stop; does not directly unblock `next_frame`. The stopper must also be called to post `WM_QUIT` so the WGC thread's `GetMessageW` returns.*
- `stopper: Option<Box<dyn FnOnce() + Send>>` - *One-shot callable that posts `WM_QUIT` to the WGC thread. Wrapped in `Option` so `take_stopper` can extract it without cloning. `None` after the stopper has been taken.*
- `drops: Arc<AtomicU64>` - *Count of frames the WGC callback couldn't hand off because the bounded queue was full (the legacy ffmpeg encoder fell behind). Shared with the callback's `Handler` via the WGC `Settings` flags; read once by `Drop::drop` to log the total.*

## try_send_or_drop

```rust
fn try_send_or_drop(tx: &SyncSender<Frame>, frame: Frame, drops: &AtomicU64)
```

Hands `frame` to the encoder without ever blocking the WGC callback thread: `tx.try_send` either enqueues it or, if the bounded queue (capacity 8) is already full, the frame is dropped and `drops` is incremented instead of the callback stalling on a blocking `send`. A free function rather than a method so it's unit-testable against a plain `sync_channel` and `AtomicU64`, with no live WGC capture session required.

### Inputs

- `tx: &SyncSender<Frame>` - The bounded channel's send half.
- `frame: Frame` - The captured frame to hand off (owned, since a dropped frame is simply discarded).
- `drops: &AtomicU64` - Incremented (`Ordering::Relaxed`) on every dropped frame; read back in `Drop::drop` for the one-time stop log.

### Returns

`()` - always succeeds; backpressure is handled by dropping, never by returning an error to the caller.

### Used by

- `session/recorder.rs` - creates a `WgcFrameSource` via `for_primary_display`, extracts `halt_handle` and `take_stopper` for coordinated shutdown, then boxes the source as `dyn FrameSource`.

## WgcFrameSource::for_primary_display

```rust
pub fn for_primary_display(clock: Arc<dyn Clock>, fps: u32, with_cursor: bool) -> anyhow::Result<Self>
```

Creates a WGC capture session targeting the primary display and returns a `WgcFrameSource` ready to serve frames.

### Inputs

- `clock: Arc<dyn Clock>` - Provides `now_ms()` inside the WGC callback. *Why injectable:* allows tests or alternate capture paths to control timestamp values without real hardware.*
- `fps: u32` - Target frame rate. *Why:* passed as `MinimumUpdateIntervalSettings::Custom(1_000_000 / fps.max(1) microseconds)` to WGC, matching the encoder's target rate so WGC does not deliver at the monitor's native refresh when that differs - a mismatch caused recordings to play at the wrong speed.*
- `with_cursor: bool` - *Selects `CursorCaptureSettings::WithCursor` or `WithoutCursor`. The TCursor synthetic-cursor path captures without the OS cursor and composites its own cursor sprite on top.*

### Implementation

1. Define an inner `Handler` struct (implements `GraphicsCaptureApiHandler`) that holds `tx`, `clock`, and `drops`. Its `on_frame_arrived` method:
   a. Reads `frame.width()` and `frame.height()`.
   b. Calls `frame.buffer()` to get the raw WGC buffer.
   c. If `buf.has_padding()`, copies `width * 4` bytes per row using `row_pitch` as the stride, building a tight `Vec<u8>`. *Why:* GPU textures may be padded to a multiple of 256 bytes per row; the encoder expects tight rows with no gaps.*
   d. Otherwise copies `as_raw_buffer()` directly.
   e. Stamps a `Timestamp(clock.now_ms())` and hands `Frame { width, height, bgra, ts }` to `try_send_or_drop(&tx, frame, &drops)`, which never blocks the callback - a full queue means the frame is dropped and counted instead of stalling capture.
2. Query `Monitor::primary()` for width/height.
3. Create a bounded mpsc `(tx, rx) = sync_channel(8)` and a fresh `drops = Arc::new(AtomicU64::new(0))`. *Why bounded:* an unbounded channel let memory grow without limit whenever the (legacy-path) ffmpeg encoder fell behind; capacity 8 caps that growth and forces an explicit drop-and-count decision instead.
4. Build `Settings` with `ColorFormat::Bgra8`, the computed `MinimumUpdateIntervalSettings`, and `(tx, clock, drops.clone())` as flags.
5. Call `Handler::start_free_threaded(settings)` to launch the WGC thread.
6. Extract `halt = control.halt_handle()` and wrap `control.stop` as the `stopper`.
7. Return `WgcFrameSource { rx, dims: (w, h), halt, stopper: Some(stopper), drops }`.

### Returns

`anyhow::Result<Self>` - fails if `Monitor::primary()`, dimension queries, `Settings` construction, or `start_free_threaded` fail.

## WgcFrameSource::halt_handle

```rust
pub fn halt_handle(&self) -> Arc<AtomicBool>
```

Returns a clone of the WGC halt handle. Callers store this before moving the source into the encoder thread, so they can signal stop from the main thread without needing a reference to the source.

### Returns

`Arc<AtomicBool>` - shared with the WGC capture session. Setting it to `true` signals the session to stop; the stopper must also be called to post `WM_QUIT`.

## WgcFrameSource::take_stopper

```rust
pub fn take_stopper(&mut self) -> Option<Box<dyn FnOnce() + Send>>
```

Extracts the one-shot stop callable, leaving `None` in its place. Returns `None` if already taken.

### Returns

`Option<Box<dyn FnOnce() + Send>>` - when `Some`, calling the box posts `WM_QUIT` to the WGC capture thread, causing it to exit `GetMessageW` and shut down promptly. Intended to be called from the main thread during `stop_recording` after setting the halt handle, because the callable must not be invoked from the encoder thread that owns the source.

## WgcFrameSource::dimensions

```rust
fn dimensions(&self) -> (u32, u32)
```

Returns the `(width, height)` of the primary monitor captured at construction. Constant for the lifetime of the source.

## WgcFrameSource::next_frame

```rust
fn next_frame(&mut self) -> Option<Frame>
```

Blocks on `self.rx.recv()` until a frame arrives or the WGC thread exits (channel closes). Returns `None` when the channel is closed, signaling the encoder loop to terminate.

## WgcFrameSource::drain_latest

```rust
fn drain_latest(&mut self) -> Option<Frame>
```

Non-blocking. Drains all pending frames from the channel using `try_recv`, discarding all but the last, and returns that last frame. Returns `None` if no frame was buffered. Used by fixed-rate pacing to avoid processing a growing backlog.

## WgcFrameSource::drop

```rust
fn drop(&mut self)
```

Safety-net diagnostic: fires once, whenever the source is torn down (owned by `RecordingSession`, so this runs right after `stop_and_finalize` returns). If `drops` is nonzero, logs `capture: dropped {n} frames (encoder behind)` to stderr so a slow legacy-path encoder is visible after the fact, without spamming per-frame during the recording itself.
