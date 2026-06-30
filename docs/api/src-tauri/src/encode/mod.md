# src-tauri/src/encode/mod.rs

MODULE OVERVIEW: The `encode` module owns the boundary between raw captured frames and the on-disk video file. It defines a push-based abstraction (`FrameSink`) that decouples the recording loop from any particular encoder backend, then provides the production implementation that pipes BGRA pixel data into a child `ffmpeg` process. The two submodules are vertically stacked: `frame_sink` defines the trait and a test double, while `ffmpeg_encoder` implements that trait for real H.264 encoding. A hardware-first encoder probe (`prewarm`) runs at app startup and caches its result, so the first recording pays no extra latency. The primary entry points for callers are `FfmpegFrameSink::new` (real-time recording) and `FfmpegFrameSink::new_hq` (offline export).

## frame_sink

Defines the `FrameSink` trait that any frame consumer must implement, and `FakeFrameSink` as a no-op test double. The `Box<Self>` receiver on `finish` enforces single-call finalization without a separate `Drop` impl, and the `Send` bound allows the boxed trait object to cross thread boundaries into the encoder thread. Key items: `FrameSink` (trait with `push` and `finish`), `FakeFrameSink` (test double recording pushed frame dimensions).

## ffmpeg_encoder

Implements `FrameSink` by spawning a child `ffmpeg` process and writing raw BGRA bytes to its stdin. Hardware encoder selection (`h264_nvenc`, `h264_qsv`, `h264_amf`, `h264_mf`, then `libx264`) is probed once at startup and cached in a `OnceLock`. Key items: `FfmpegFrameSink` (active encode session wrapping the child process), `prewarm` (triggers the hardware probe and logs diagnostics), `FfmpegFrameSink::new` (real-time recording quality), `FfmpegFrameSink::new_hq` (high-quality offline export with `f64` fps).
