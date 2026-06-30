# src-tauri/src/encode/frame_sink.rs

Defines the `FrameSink` trait for push-based frame consumption and `FakeFrameSink` as a test double that records received frame dimensions. The trait is the boundary between the encoder loop and any backend (ffmpeg process, null sink, file writer); `Send` on the trait allows a `Box<dyn FrameSink>` to be moved to the encoder thread. The `Box<Self>` receiver on `finish` enforces single-call finalization without `Drop`.

## FrameSink

```rust
pub trait FrameSink: Send {
    fn push(&mut self, f: &Frame) -> std::io::Result<()>;
    fn finish(self: Box<Self>) -> std::io::Result<()>;
}
```

Abstraction over any consumer of captured frames.

- `push(&mut self, f: &Frame) -> std::io::Result<()>` - *Accepts one frame for processing. Returns `Ok(())` on success or an `io::Error` on failure. Called in a tight loop by the encoder thread; a returned error is treated as fatal by the caller.*
- `finish(self: Box<Self>) -> std::io::Result<()>` - *Signals that no more frames will arrive and performs finalization (flushing, waiting for a child process, writing headers). Takes `Box<Self>` to release the sink's resources without requiring a separate `Drop` impl. Must be called exactly once after the last `push`.*

### Used by

- `encode/ffmpeg_encoder.rs` - `FfmpegFrameSink` implements `FrameSink`; `push` writes BGRA bytes to ffmpeg's stdin; `finish` closes stdin and waits for the child process.
- `session/recording_session.rs` - stores `Box<dyn FrameSink>` and calls `push` per frame, `finish` on stop.
- `session/pacing.rs` - receives `&mut dyn FrameSink` and calls `push` in both event-driven and fixed-rate modes.
- `export/exporter.rs` - creates a `FfmpegFrameSink::new_hq` and passes it as `dyn FrameSink` to the offline re-encode loop.

## FakeFrameSink

```rust
#[derive(Default)]
pub struct FakeFrameSink {
    pub pushed: Vec<(u32, u32)>,
}
```

Test double. Records the `(width, height)` of every frame passed to `push` in order. `finish` is a no-op.

- `pushed: Vec<(u32, u32)>` - *Accumulates `(f.width, f.height)` for each call to `push`. `pub` so tests can assert on it directly after the loop under test completes.*

### Used by

- `encode/frame_sink.rs` (tests) - `fake_sink_records_pushes_and_finish`.
- `session/recording_session.rs` (tests) - all recording-session unit tests inject a `FakeFrameSink` to capture which frames were delivered.
- `session/pacing.rs` (tests) - pacing tests verify frame counts via `FakeFrameSink.pushed.len()`.

### Behaviors

- `fake_sink_records_pushes_and_finish` - creates a `Frame { width: 4, height: 2, ... }`, calls `push`, asserts `pushed == [(4, 2)]`, then calls `Box::new(sink).finish()` and asserts it returns `Ok(())`.
