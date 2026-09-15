# src-tauri/src/platform/windows/capture/legacy/wgc_source.rs

Bridges the Windows Graphics Capture (WGC) API's callback-based frame delivery into the pull-based `capture::frame_source::FrameSource` interface via an internal mpsc channel. The key properties are: WGC runs its callback on its own thread (WGC thread); the encoder thread pulls frames via `next_frame` or `drain_latest`; and row padding is stripped inside the callback so consumers receive tightly-packed BGRA rows. Shutdown requires two coordinated steps - set the halt handle and call the stopper - to avoid leaking the WGC thread.

This is the legacy path's frame source only. The default GPU path never reads a frame back to the CPU at all; see `gpu/frames.rs`. It was `capture/windows_capture.rs` until Batch C1; `crate::capture::windows_capture` is still a working path, as a `#[cfg(windows)]` module re-export in `capture/mod.rs`, so `tests/manual_capture.rs` and anything else naming the old path keeps compiling until Batch D.

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
- `dims: (u32, u32)` - *The capture target's nominal size, resolved once at construction and returned by `dimensions()` without waiting for a frame. An ESTIMATE: for a window it is the outer `GetWindowRect`, which includes the invisible DWM resize margins the capture surface does not have. The legacy encoder is sized from it anyway, which is why a mid-take size change ends this path's take (`DISPLAY_CHANGED`) rather than being fitted the way the GPU path fits it.*
- `halt: Arc<AtomicBool>` - *WGC halt handle. Setting it to `true` signals the capture session to stop; does not directly unblock `next_frame`. The stopper must also be called to post `WM_QUIT` so the WGC thread's `GetMessageW` returns.*
- `stopper: Option<Box<dyn FnOnce() + Send>>` - *One-shot callable that posts `WM_QUIT` to the WGC thread. Wrapped in `Option` so `take_stopper` can extract it without cloning. `None` after the stopper has been taken.*
- `drops: Arc<AtomicU64>` - *Count of frames the WGC callback couldn't hand off because the bounded queue was full (the ffmpeg encoder fell behind). Shared with the callback's `Handler` via the WGC `Settings` flags; read once by `Drop::drop` to log the total.*

## Flags

```rust
type Flags = (SyncSender<Frame>, Arc<dyn Clock>, Arc<AtomicU64>);
```

What WGC carries from `Settings` into `Handler::new`: the channel's send half, the recording clock and the shared drop counter. Named because `launch` and the handler both have to spell it, and a bare three-tuple in two signatures is where they would drift apart.

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

- `platform/windows/capture/legacy/mod.rs` - `start_ffmpeg` creates a `WgcFrameSource` via `for_target`, extracts `halt_handle` and `take_stopper` for coordinated shutdown, then boxes the source as `dyn FrameSource`.

## Handler

```rust
struct Handler { tx: SyncSender<Frame>, clock: Arc<dyn Clock>, drops: Arc<AtomicU64> }
```

The `GraphicsCaptureApiHandler` WGC runs on its capture thread. At module scope rather than nested inside `for_target`, so `launch` can name it and all three target branches go through one `start_free_threaded` call.

## Handler::new

```rust
fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error>
```

Unpacks the `Flags` tuple WGC carried through `Settings`. Nothing else: the handler holds no D3D state because this path reads frames back to the CPU.

## Handler::on_frame_arrived

```rust
fn on_frame_arrived(&mut self, frame: &mut WgcFrame, _ctl: InternalCaptureControl) -> Result<(), Self::Error>
```

Copies one frame out of the GPU buffer and hands it to the channel.

1. Reads `frame.width()` / `frame.height()` and calls `frame.buffer()`.
2. If `buf.has_padding()`, copies `width * 4` bytes per row using `row_pitch` as the stride, building a tight `Vec<u8>`. *Why:* GPU textures may be padded to a multiple of 256 bytes per row; the encoder expects tight rows with no gaps.
3. Otherwise copies `as_raw_buffer()` directly.
4. Stamps a `Timestamp(clock.now_ms())` and hands `Frame { width, height, bgra, ts }` to `try_send_or_drop`, which never blocks the callback - a full queue means the frame is dropped and counted instead of stalling capture.

## launch

```rust
fn launch<T: TryIntoCaptureItemWithType + Send + 'static>(item: T, cursor: CursorCaptureSettings, interval: MinimumUpdateIntervalSettings, flags: Flags) -> anyhow::Result<(Arc<AtomicBool>, Box<dyn FnOnce() + Send>)>
```

Starts one WGC capture of `item` and returns its two shutdown handles: the halt flag and the one-shot `WM_QUIT` stopper. The single place a `Settings` is built on this path - monitors, windows and the primary-display fallback differ only in the item they hand in, so the colour format, the border setting and the update-interval floor cannot drift apart between the three. Generic over the item type because `Settings` is: `Monitor` and `Window` share no common type, only `TryIntoCaptureItemWithType`. `flags` is taken by value and moved exactly once per branch, which is why each branch returns rather than falling through after a launch.

Mirrors `gpu::record::launch`, deliberately: the two paths must open the same capture, and the only difference is what they do with the frames.

## primary

```rust
fn primary(cursor: CursorCaptureSettings, interval: MinimumUpdateIntervalSettings, flags: Flags) -> anyhow::Result<((u32, u32), Arc<AtomicBool>, Box<dyn FnOnce() + Send>)>
```

`Monitor::primary()` plus `launch`. A named function rather than a fall-through tail because it is reached from two places: `TargetId::Primary`, and a `display:N` whose index no longer resolves to a monitor - which is what the hand-written parser this replaced did by falling out of its `if let` chain.

## WgcFrameSource::for_primary_display

```rust
pub fn for_primary_display(clock: Arc<dyn Clock>, fps: u32, with_cursor: bool) -> anyhow::Result<Self>
```

`for_target(clock, fps, with_cursor, None)`. Kept as its own name for `tests/manual_capture.rs`, which records the primary display and has no target to name.

## WgcFrameSource::for_target

```rust
pub fn for_target(clock: Arc<dyn Clock>, fps: u32, with_cursor: bool, target_id: Option<&str>) -> anyhow::Result<Self>
```

Creates a WGC capture session on `target_id` and returns a `WgcFrameSource` ready to serve frames.

### Inputs

- `clock: Arc<dyn Clock>` - Provides `now_ms()` inside the WGC callback. *Why injectable:* allows tests or alternate capture paths to control timestamp values without real hardware.
- `fps: u32` - Target frame rate. *Why:* passed as `MinimumUpdateIntervalSettings::Custom(1_000_000 / fps.max(1) microseconds)` to WGC, matching the encoder's target rate so WGC does not deliver at the monitor's native refresh when that differs - a mismatch caused recordings to play at the wrong speed.
- `with_cursor: bool` - *Selects `CursorCaptureSettings::WithCursor` or `WithoutCursor`. Always `false` in TCursor now (`recorder.rs`): the display is captured clean for every cursor style and the real OS cursor is recorded as its own layer (`events::track::cursorlayer`), which the export composites for "System" and the synthetic sprite stack replaces for "Enhanced". The parameter stays because the capture layer should not hardcode a product decision.*
- `target_id: Option<&str>` - Read once through `ports::capture::TargetId::from_arg`, never parsed by hand. `window:0x<hex>` captures that HWND at `target::capture_window_size`; `display:N` captures that monitor; `None`, `"primary"` and anything unparseable capture the primary display, which is exactly what the four hand-written parsers did by falling through.

### Implementation

1. Create a bounded mpsc `(tx, rx) = sync_channel(8)` and a fresh `drops`. *Why bounded:* an unbounded channel let memory grow without limit whenever the ffmpeg encoder fell behind; capacity 8 caps that growth and forces an explicit drop-and-count decision instead.
2. Resolve the cursor and update-interval settings from `with_cursor` and `fps`.
3. Match the `TargetId` to a capture item, take its nominal size, and `launch` it - or `primary` when there is no item.
4. Return `WgcFrameSource { rx, dims, halt, stopper: Some(stopper), drops }`.

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

Returns the `(width, height)` of the capture target resolved at construction. Constant for the lifetime of the source, which is what makes a mid-take size change fatal on this path.

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

Safety-net diagnostic: fires once, whenever the source is torn down (owned by `RecordingSession`, so this runs right after `stop_and_finalize` returns). If `drops` is nonzero, logs `capture: dropped {n} frames (encoder behind)` to stderr so a slow encoder is visible after the fact, without spamming per-frame during the recording itself.
