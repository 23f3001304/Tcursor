# src-tauri/src/encode/mod.rs

MODULE OVERVIEW: The `encode` module owns the boundary between raw captured frames and the on-disk video file. It defines a push-based abstraction (`FrameSink`) that decouples the recording loop from any particular encoder backend, then provides the production implementation that pipes BGRA pixel data into a child `ffmpeg` process. `frame_sink` defines the trait and a test double; `ffmpeg_encoder` implements that trait (real-time H.264 recording via `new`/`new_vfr`, plus offline export via `new_medium`); `ffmpeg_args` holds the pure, unit-tested argument construction `new_medium` uses to support MP4/H.264, WebM/VP9, and GIF export formats. A hardware-first H.264 encoder probe (`prewarm`) runs at app startup and caches its result, so the first recording pays no extra latency. The primary entry points for callers are `FfmpegFrameSink::new`/`new_vfr` (real-time recording) and `FfmpegFrameSink::new_medium` (offline export, `exporter::export`'s only caller).

## frame_sink

Defines the `FrameSink` trait that any frame consumer must implement, and `FakeFrameSink` as a no-op test double. The `Box<Self>` receiver on `finish` enforces single-call finalization without a separate `Drop` impl, and the `Send` bound allows the boxed trait object to cross thread boundaries into the encoder thread. Key items: `FrameSink` (trait with `push` and `finish`), `FakeFrameSink` (test double recording pushed frame dimensions).

## ffmpeg_encoder

Implements `FrameSink` by spawning a child `ffmpeg` process and writing raw BGRA bytes to its stdin. Hardware encoder selection (`h264_nvenc`, `h264_qsv`, `h264_amf`, `h264_mf`, then `libx264`) is probed once at startup and cached in a `OnceLock`. Key items: `FfmpegFrameSink` (active encode session wrapping the child process), `prewarm` (triggers the hardware probe and logs diagnostics), `FfmpegFrameSink::new` (real-time recording quality), `FfmpegFrameSink::new_hq` (high-quality offline export with `f64` fps), `FfmpegFrameSink::new_medium` (offline export - `exporter::export`'s only caller - now format/quality-aware via `ffmpeg_args::export_args`).

## ffmpeg_args

Pure ffmpeg CLI argument construction for one export encode, split out so the settings -> args mapping is unit-tested without spawning ffmpeg. Key items: `export_args(format, h264_encoder, width, height, fps, crf, out_path) -> Vec<String>` - dispatches on `export::settings::Format` to build MP4/H.264 (crf-aware for `libx264`/`h264_nvenc` only), WebM/VP9 (constant-quality), or GIF (palettegen/paletteuse) arguments.
