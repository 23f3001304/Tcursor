# src-tauri/src/capture/frame.rs

Defines `Frame`, the fundamental unit of captured screen data exchanged between the capture and encode layers. The struct is pure data with no methods - it is allocated on the capture thread by the WGC callback and consumed on the encoder thread. Row padding is stripped before construction so `bgra` is always tightly packed.

## Frame

```rust
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub ts: Timestamp,
}
```

One captured screen frame in BGRA8 format (Blue-Green-Red-Alpha, 4 bytes per pixel, row-major, no row padding).

- `width: u32` - *Frame width in pixels. Together with `height`, lets any consumer verify the buffer length and correctly stride into rows without querying the source again.*
- `height: u32` - *Frame height in pixels.*
- `bgra: Vec<u8>` - *Raw pixel bytes. Expected length is `width * height * 4`. The capture layer (`windows_capture.rs`) strips row-padding before creating this `Vec`, so bytes form tight rows with no stride gaps. ffmpeg's `-f rawvideo -pixel_format bgra` pipeline expects exactly this layout.*
- `ts: Timestamp` - *Wall-clock millisecond timestamp from the recording session's `Clock`, stamped at callback delivery time. Used for A/V sync decisions, frame ordering, and computing inter-frame durations.*

### Used by

- `capture/windows_capture.rs` - constructs `Frame` values in the WGC callback after stripping row padding; sends them over the mpsc channel.
- `capture/frame_source.rs` - `FrameSource::next_frame` and `drain_latest` deliver `Frame` values to the encoder; `FakeFrameSource` holds pre-loaded frames for tests.
- `encode/frame_sink.rs` - `FrameSink::push` receives `&Frame`; `FakeFrameSink` records `(width, height)` pairs for test assertions.
- `encode/ffmpeg_encoder.rs` - `FfmpegFrameSink::push` writes `f.bgra` directly to ffmpeg's stdin.
- `session/pacing.rs` and `session/recording_session.rs` - thread the `Frame` between source and sink in the encoder loop.
- `export/exporter.rs` - reuses `Frame` during offline re-encode.
