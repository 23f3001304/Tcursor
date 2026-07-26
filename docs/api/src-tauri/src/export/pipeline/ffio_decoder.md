# src-tauri/src/export/pipeline/ffio_decoder.rs

`RawDecoder`: a spawned ffmpeg process emitting a continuous stream of raw BGRA frames. Split out of `ffio.rs` (which keeps the ffprobe/decode/crop free-function helpers) purely to stay under the file line-count limit; `ffio.rs` re-exports this as `ffio::RawDecoder`, so every existing import path (`crate::export::pipeline::ffio::RawDecoder`) is unchanged.

## RawDecoder

```rust
pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
}
```

A spawned ffmpeg process emitting a continuous stream of raw BGRA frames at a fixed byte size per frame. The process is kept alive until the decoder is dropped.

### Fields

- `child` - *the spawned ffmpeg process; killed and waited on `Drop` to avoid zombie processes.*
- `stdout` - *the process's stdout pipe; `read_exact` on this reads exactly one frame at a time.*
- `frame_bytes` - *expected bytes per frame (`width * height * 4`); asserted in `read_frame` to detect size mismatches early.*

### Used by

- `src-tauri/src/export/pipeline/mod.rs` - `ScreenPipe::spawn` and `WebcamPipe::spawn` each spawn one `RawDecoder`.
- `src-tauri/src/export/pipeline/pipeline_decode.rs` - `spawn_screen`/`spawn_webcam` decode-thread bodies read frames from a `RawDecoder` in a loop.
- `src-tauri/src/export/preview/mod.rs` - the preview engine spawns short-lived decoders directly to grab a single frame at an arbitrary time.

## RawDecoder::spawn

```rust
pub fn spawn(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, square_scale: Option<u32>, target_dims: Option<(u32, u32)>, frame_bytes: usize) -> Result<Self>
```

Spawns the ffmpeg decoder subprocess and captures its stdout.

### Inputs

- `video: &Path` - video file to decode. *Why:* the primary ffmpeg input.*
- `rate: f64` - target frame rate; `<= 0` decodes at the native rate. *Why:* the exporter controls timing and may need frames at a specific rate for sync.*
- `input_rate: bool` - if `true`, `-r rate` is placed before `-i` (input demux rate override); if `false`, after (output filter rate). *Why:* certain container formats need the input rate set to suppress duplicate-frame detection; others need the output filter.*
- `seek_ms: Option<u64>` - trim start offset in ms (`-ss`). *Why:* using ffmpeg's native seek is orders of magnitude faster than decoding and discarding frames.*
- `square_scale: Option<u32>` - if set, cover-crops the video to a centered square of this size. *Why:* webcam feeds are 16:9; squaring avoids aspect-ratio distortion in the picture-in-picture overlay.*
- `target_dims: Option<(u32, u32)>` - if set, scales output to these exact dimensions via an FFmpeg `-vf scale`. *Why:* lets the screen decoder resize directly in FFmpeg (e.g. to the adapted output resolution) instead of a separate CPU resize pass. Mutually exclusive with `square_scale` in practice - `target_dims` is checked first.*
- `frame_bytes: usize` - expected bytes per frame. *Why:* stored for the `debug_assert` in `read_frame`.*

### Returns

`Result<Self>` - the running decoder. Errors if the spawn fails or stdout is unavailable.

### Implementation

1. Build ffmpeg command: `-v error -hwaccel auto`, optional `-ss seek_ms/1000.0`, optional input `-r rate`, `-i video`, optional output `-r rate`, `-sws_flags fast_bilinear`, then either `-vf scale=w:h:flags=fast_bilinear` (`target_dims`) or `-vf scale=sz:sz:force_original_aspect_ratio=increase,crop=sz:sz:flags=fast_bilinear` (`square_scale`), `-f rawvideo -pix_fmt bgra`, `-` (stdout). Stderr suppressed.
2. Take `child.stdout`; return `Self { child, stdout, frame_bytes }`.

## RawDecoder::read_frame

```rust
pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool>
```

Reads exactly one frame into `buf` from the ffmpeg stdout pipe.

### Inputs

- `buf: &mut [u8]` - caller-owned buffer of exactly `frame_bytes`. *Why caller-owned:* avoids a per-frame allocation in the hot render loop.*

### Returns

`Result<bool>` - `Ok(true)` when a full frame was read; `Ok(false)` at end-of-stream (`UnexpectedEof`); `Err` on any other I/O error.

### Implementation

1. `self.stdout.read_exact(buf)` -> `Ok(true)` on success, `Ok(false)` on `UnexpectedEof`, `Err` otherwise.
