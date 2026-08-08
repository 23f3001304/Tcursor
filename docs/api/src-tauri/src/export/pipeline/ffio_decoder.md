# src-tauri/src/export/pipeline/ffio_decoder.rs

`RawDecoder`: a spawned ffmpeg process emitting a continuous stream of raw frames in a caller-chosen pixel format (`bgra` for the webcam, `nv12` for the screen - the latter is ~2.6x smaller through the pipe and skips ffmpeg's yuv->bgra convert, the main export-throughput win). Split out of `ffio.rs` (which keeps the ffprobe/decode/crop free-function helpers) purely to stay under the file line-count limit; `ffio.rs` re-exports this as `ffio::RawDecoder`, so every existing import path (`crate::export::pipeline::ffio::RawDecoder`) is unchanged.

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
- `frame_bytes` - *expected bytes per frame (`width * height * 4` for bgra, `width * height * 3 / 2` for nv12); asserted in `read_frame` to detect size mismatches early.*

### Used by

- `src-tauri/src/export/pipeline/mod.rs` - `ScreenPipe::spawn` and `WebcamPipe::spawn` each spawn one `RawDecoder`.
- `src-tauri/src/export/pipeline/pipeline_decode.rs` - `spawn_screen`/`spawn_webcam` decode-thread bodies read frames from a `RawDecoder` in a loop.
- `src-tauri/src/export/preview/mod.rs` - the preview engine spawns short-lived decoders directly to grab a single frame at an arbitrary time.

## RawDecoder::spawn

```rust
pub fn spawn(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, square_scale: Option<u32>, target_dims: Option<(u32, u32)>, pix_fmt: &str, frame_bytes: usize) -> Result<Self>
```

Spawns the ffmpeg decoder subprocess and captures its stdout.

### Inputs

- `video: &Path` - video file to decode. *Why:* the primary ffmpeg input.*
- `rate: f64` - target frame rate; `<= 0` decodes at the native rate. *Why:* the exporter controls timing and may need frames at a specific rate for sync.*
- `input_rate: bool` - if `true`, `-r rate` is placed before `-i` (input demux rate override); if `false`, after (output filter rate). *Why:* certain container formats need the input rate set to suppress duplicate-frame detection; others need the output filter.*
- `seek_ms: Option<u64>` - trim start offset in ms (`-ss`). *Why:* using ffmpeg's native seek is orders of magnitude faster than decoding and discarding frames.*
- `square_scale: Option<u32>` - if set, cover-crops the video to a centered square of this size. *Why:* webcam feeds are 16:9; squaring avoids aspect-ratio distortion in the picture-in-picture overlay.*
- `target_dims: Option<(u32, u32)>` - if set, scales output to these exact dimensions via an FFmpeg `-vf scale`. *Why:* lets the screen decoder resize directly in FFmpeg (e.g. to the adapted output resolution) instead of a separate CPU resize pass. Mutually exclusive with `square_scale` in practice - `target_dims` is checked first.*
- `pix_fmt: &str` - output raw pixel format (`"bgra"` or `"nv12"`). *Why:* the screen decodes as `nv12` (Y + interleaved half-res UV) so far fewer bytes cross the pipe and ffmpeg skips the yuv->bgra convert (the GPU/CPU compositor converts instead); the webcam stays `bgra`.*
- `frame_bytes: usize` - expected bytes per frame (matches `pix_fmt`: `w*h*4` bgra, `w*h*3/2` nv12). *Why:* stored for the `debug_assert` in `read_frame`.*

### Returns

`Result<Self>` - the running decoder. Errors if the spawn fails or stdout is unavailable.

### Implementation

1. Build the arg list via `decode_args` (below); stderr suppressed.
2. Take `child.stdout`; return `Self { child, stdout, frame_bytes }`.

## decode_args

```rust
fn decode_args(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, square_scale: Option<u32>, target_dims: Option<(u32, u32)>, pix_fmt: &str) -> Vec<String>
```

Pure builder for `RawDecoder::spawn`'s ffmpeg arg list - no process spawn, so the `-r`/`-vf`/`-pix_fmt` selection is unit-testable directly. Mirrors the exact arg order the command used to be built inline: `-v error -hwaccel auto`, optional `-ss seek_ms/1000.0`, optional input `-r rate` (`input_rate == true`), `-i video`, optional output `-r rate` (`input_rate == false`), `-sws_flags fast_bilinear`, then either `-vf scale=w:h:flags=fast_bilinear` (`target_dims`) or `-vf scale=sz:sz:force_original_aspect_ratio=increase,crop=sz:sz` (`square_scale`), `-f rawvideo -pix_fmt <pix_fmt>`, `-` (stdout).

### Inputs

Same as `RawDecoder::spawn` minus `frame_bytes` (that field is stored on `RawDecoder`, not needed to build the ffmpeg args).

### Returns

`Vec<String>` - the full ffmpeg argument list (everything after the `ffmpeg` binary name itself).

### Behaviors

- `positive_output_rate_emits_r_after_i` - `rate > 0.0, input_rate = false` (the screen/webcam decode's shape) places `-r <rate>` AFTER `-i`, formatted `%.4f`.
- `positive_input_rate_emits_r_before_i` - `input_rate = true` places `-r` BEFORE `-i` instead.
- `non_positive_rate_omits_r_entirely` - `rate <= 0.0` (native-rate decode, e.g. the preview engine's seek-one-frame calls) emits no `-r` at all.

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
