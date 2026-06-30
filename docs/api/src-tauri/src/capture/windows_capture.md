# src-tauri/src/capture/windows_capture.rs

Bridges the Windows Graphics Capture (WGC) API's callback-based frame delivery into the pull-based `FrameSource` interface via an internal mpsc channel. The key properties are: WGC runs its callback on its own thread (WGC thread); the encoder thread pulls frames via `next_frame` or `drain_latest`; and row padding is stripped inside the callback so consumers receive tightly-packed BGRA rows. Shutdown requires two coordinated steps - set the halt handle and call the stopper - to avoid leaking the WGC thread.

## WgcFrameSource

```rust
pub struct WgcFrameSource {
    rx: Receiver<Frame>,
    dims: (u32, u32),
    halt: Arc<AtomicBool>,
    stopper: Option<Box<dyn FnOnce() + Send>>,
}
```

Implements `FrameSource` on top of a WGC capture session running on its own OS thread.

- `rx: Receiver<Frame>` - *Pull end of the mpsc channel. `next_frame` blocks on `recv()`; `drain_latest` drains with `try_recv`. When the WGC thread exits and drops the `Sender`, `recv()` returns `Err`, causing `next_frame` to return `None` and the encoder loop to exit.*
- `dims: (u32, u32)` - *Primary monitor dimensions captured once at construction. Returned by `dimensions()` without waiting for a frame.*
- `halt: Arc<AtomicBool>` - *WGC halt handle. Setting it to `true` signals the capture session to stop; does not directly unblock `next_frame`. The stopper must also be called to post `WM_QUIT` so the WGC thread's `GetMessageW` returns.*
- `stopper: Option<Box<dyn FnOnce() + Send>>` - *One-shot callable that posts `WM_QUIT` to the WGC thread. Wrapped in `Option` so `take_stopper` can extract it without cloning. `None` after the stopper has been taken.*

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

1. Define an inner `Handler` struct (implements `GraphicsCaptureApiHandler`) that holds `tx` and `clock`. Its `on_frame_arrived` method:
   a. Reads `frame.width()` and `frame.height()`.
   b. Calls `frame.buffer()` to get the raw WGC buffer.
   c. If `buf.has_padding()`, copies `width * 4` bytes per row using `row_pitch` as the stride, building a tight `Vec<u8>`. *Why:* GPU textures may be padded to a multiple of 256 bytes per row; the encoder expects tight rows with no gaps.*
   d. Otherwise copies `as_raw_buffer()` directly.
   e. Stamps a `Timestamp(clock.now_ms())` and sends `Frame { width, height, bgra, ts }` on `tx`. Send errors are silently discarded - the receiver may have shut down.
2. Query `Monitor::primary()` for width/height.
3. Create an mpsc `(tx, rx)` channel.
4. Build `Settings` with `ColorFormat::Bgra8`, the computed `MinimumUpdateIntervalSettings`, and `(tx, clock)` as flags.
5. Call `Handler::start_free_threaded(settings)` to launch the WGC thread.
6. Extract `halt = control.halt_handle()` and wrap `control.stop` as the `stopper`.
7. Return `WgcFrameSource { rx, dims: (w, h), halt, stopper: Some(stopper) }`.

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
