# src-tauri/src/capture/frame_source.rs

Defines the `FrameSource` trait for pull-based frame delivery and `FakeFrameSource` as a deterministic test double backed by a `VecDeque`. The trait is the boundary between any capture backend and the encoder loop - backends implement it, the encoder calls it. Thread safety is encoded in the `Send` bound; the encoder thread owns the source exclusively after construction.

## FrameSource

```rust
pub trait FrameSource: Send {
    fn dimensions(&self) -> (u32, u32);
    /// Returns the next frame, or None when the source has ended.
    fn next_frame(&mut self) -> Option<Frame>;
    /// Newest frame available right now, discarding any older buffered frames;
    /// None if none is ready. Non-blocking. Used by fixed-rate (game) pacing.
    fn drain_latest(&mut self) -> Option<Frame>;
}
```

Abstraction over any source of captured `Frame`s. Implementors must be `Send` so they can be moved to the encoder thread.

- `dimensions()` - Returns `(width, height)` in pixels. *Why:* the encoder needs dimensions at startup to configure the video stream header before any frame arrives, without waiting for the first frame.*
- `next_frame()` - Blocking pull. Returns the next available `Frame` or `None` when the source has ended (e.g. the underlying channel's sender was dropped). *Why blocking:* event-driven pacing waits on the source so the encoder thread does not spin.*
- `drain_latest()` - Non-blocking pull. Discards all buffered frames except the newest and returns that frame, or `None` if no frame is currently buffered. *Why:* fixed-rate (game-loop) pacing always works with the freshest image rather than consuming a backlog in order, keeping the video from falling behind real time.*

### Used by

- `capture/windows_capture.rs` - `WgcFrameSource` implements `FrameSource` over an mpsc channel fed by the WGC callback thread.
- `session/pacing.rs` - calls `next_frame` and `drain_latest` depending on pacing mode.
- `session/recording_session.rs` - stores `Box<dyn FrameSource>` and drives the encode loop.
- `session/recorder.rs` - constructs a `WgcFrameSource` and boxes it as `dyn FrameSource`.

## FakeFrameSource

```rust
pub struct FakeFrameSource {
    frames: std::collections::VecDeque<Frame>,
    dims: (u32, u32),
}
```

Test double backed by a `VecDeque` of pre-loaded `Frame`s.

- `frames: VecDeque<Frame>` - *Queue of frames to be returned by `next_frame` in insertion order. `drain_latest` empties the queue and returns the last element.*
- `dims: (u32, u32)` - *Captured from the first frame's `width`/`height` at construction; defaults to `(0, 0)` for an empty input. Returned by `dimensions()` without inspecting the queue again.*

### Used by

- `capture/frame_source.rs` (tests) - `fake_source_drains_in_order_then_ends`, `drain_latest_returns_newest_and_empties`.
- `session/recording_session.rs` (tests) - all recording-session tests inject a `FakeFrameSource`.
- `session/pacing.rs` (tests) - pacing tests inject a `FakeFrameSource`.

## FakeFrameSource::new

```rust
pub fn new(frames: Vec<Frame>) -> Self
```

Constructs a `FakeFrameSource` from an owned `Vec<Frame>`. The frames are moved into a `VecDeque` for O(1) front-removal by `next_frame`. Dimensions are inferred from the first frame.

### Inputs

- `frames: Vec<Frame>` - Pre-loaded frames to serve. *Why `Vec` not `VecDeque`:* callers construct with a `vec![]` literal; the conversion is O(n) but only happens once at test setup.*

### Returns

`Self` with `dims` derived from `frames.first()` (or `(0, 0)` if empty).

### Behaviors

- `fake_source_drains_in_order_then_ends` - a two-frame source returns frames at timestamps 0 and 33 in order, then returns `None`.
- `drain_latest_returns_newest_and_empties` - a three-frame source returns the frame at timestamp 33 (newest), and a subsequent `drain_latest` returns `None` (queue is empty).
