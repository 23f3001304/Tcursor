# src-tauri/src/export/pipeline/ffio_decoder.rs

`RawDecoder`: a spawned ffmpeg process emitting a continuous stream of raw frames in a caller-chosen pixel format (`bgra` for the webcam, `nv12` for the screen - the latter is ~2.6x smaller through the pipe and skips ffmpeg's yuv->bgra convert, the main export-throughput win). Split out of `ffio.rs` (which keeps the ffprobe/decode/crop free-function helpers) purely to stay under the file line-count limit; `ffio.rs` re-exports this as `ffio::RawDecoder`, so every existing import path (`crate::export::pipeline::ffio::RawDecoder`) is unchanged.

## RawDecoder

```rust
pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
    stderr: Arc<Mutex<String>>,
    drain: Option<JoinHandle<()>>,
    frames: u64,
}
```

A spawned ffmpeg process emitting a continuous stream of raw BGRA frames at a fixed byte size per frame. The process is kept alive until the decoder is dropped.

### Fields

- `child` - *the spawned ffmpeg process; killed and waited on `Drop` to avoid zombie processes.*
- `stdout` - *the process's stdout pipe; `read_exact` on this reads exactly one frame at a time.*
- `frame_bytes` - *expected bytes per frame (`width * height * 4` for bgra, `width * height * 3 / 2` for nv12); asserted in `read_frame` to detect size mismatches early.*
- `stderr` - *rolling tail (last ~2 KB, `TAIL_CAP`) of ffmpeg's own stderr, filled by the `drain` thread. Shared behind `Arc<Mutex<..>>` because it is written from that thread and read from the frame reader.*
- `drain` - *join handle of the stderr-draining thread; joined once (and taken) in `end_of_stream` so the tail is complete before it is reported. `None` after that, or if the pipe could not be taken.*
- `frames` - *frames successfully read so far. Reported in the failure message: it separates "ffmpeg died before producing anything" (a rejected filtergraph, an unreadable file) from "ffmpeg died part-way through".*

### Used by

- `src-tauri/src/export/pipeline/mod.rs` - `ScreenPipe::spawn` and `WebcamPipe::spawn` each spawn one `RawDecoder`.
- `src-tauri/src/export/pipeline/bg_pipe.rs` - `BgPipe::open` spawns one through `spawn_args`, with its own looping arg list.
- `src-tauri/src/export/pipeline/mod.rs` - the `spawn_screen`/`spawn_webcam` decode-thread bodies read frames from a `RawDecoder` in a loop.
- `src-tauri/src/export/preview/mod.rs` - the preview engine spawns short-lived decoders directly to grab a single frame at an arbitrary time.

## RawDecoder::spawn

```rust
pub fn spawn(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, crop: Option<(u32, u32)>, cover_scale: Option<(u32, u32)>, target_dims: Option<(u32, u32)>, pix_fmt: &str, frame_bytes: usize) -> Result<Self>
```

Spawns the ffmpeg decoder subprocess and captures its stdout.

### Inputs

- `video: &Path` - video file to decode. *Why:* the primary ffmpeg input.*
- `rate: f64` - target frame rate; `<= 0` decodes at the native rate. *Why:* the exporter controls timing and may need frames at a specific rate for sync.*
- `input_rate: bool` - if `true`, `-r rate` is placed before `-i` (input demux rate override); if `false`, after (output filter rate). *Why:* certain container formats need the input rate set to suppress duplicate-frame detection; others need the output filter.*
- `seek_ms: Option<u64>` - trim start offset in ms (`-ss`). *Why:* using ffmpeg's native seek is orders of magnitude faster than decoding and discarding frames.*
- `crop: Option<(u32, u32)>` - if set, an EXACT `crop=w:h:0:0` (no resample) applied before any scale. *Why:* its one use is trimming an odd-sized capture to even dims (`render::meta::even_screen`): nv12 has no odd sizes, ffmpeg pads the chroma plane, and the frame no longer measures `w*h*3/2` bytes - so every frame read off the pipe started a fraction of a row late and the whole export slid and sheared (2026-09-14). Dropping one column or row of a screen capture is invisible; resampling it would not be.*
- `cover_scale: Option<(u32, u32)>` - if set, cover-crops the video to a centered `w`x`h` box (scale-up to cover, then crop). *Why a pair rather than one square side:* the webcam decode box (`RenderMeta::webcam_w`/`webcam_h`, from `render::meta::webcam_box`) carries the SOURCE's aspect so that the compositors can cover-crop it to whichever panel aspect the layout is showing; forcing a square here threw the sides of a 16:9 webcam away before any panel could ask for them. Equal dims reproduce the old square exactly.*
- `target_dims: Option<(u32, u32)>` - if set, scales output to these exact dimensions via an FFmpeg `-vf scale`. *Why:* lets the screen decoder resize directly in FFmpeg (e.g. to the adapted output resolution) instead of a separate CPU resize pass. Mutually exclusive with `cover_scale` in practice - `target_dims` is checked first, and unlike `cover_scale` it does NOT preserve the source aspect.*
- `pix_fmt: &str` - output raw pixel format (`"bgra"` or `"nv12"`). *Why:* the screen decodes as `nv12` (Y + interleaved half-res UV) so far fewer bytes cross the pipe and ffmpeg skips the yuv->bgra convert (the GPU/CPU compositor converts instead); the webcam stays `bgra`.*
- `frame_bytes: usize` - expected bytes per frame (matches `pix_fmt`: `w*h*4` bgra, `w*h*3/2` nv12). *Why:* stored for the `debug_assert` in `read_frame`.*

### Returns

`Result<Self>` - the running decoder. Errors if the spawn fails or stdout is unavailable.

### Implementation

1. Build the arg list via `decode_args` (below) and hand it to `spawn_args`.

## RawDecoder::spawn_args

```rust
pub fn spawn_args(args: Vec<String>, frame_bytes: usize) -> Result<Self>
```

`spawn` for a caller that builds its OWN argument list: the background stream (`pipeline::bg_pipe`), whose `-stream_loop -1 -an` shape does not fit `decode_args`' parameters and should not distort them for the three call sites that do fit. Everything downstream is shared and identical - which is the point of the split.

1. Spawn `ffmpeg` with `args`. stdout AND stderr are piped - stderr is *captured*, not discarded (it used to be `Stdio::null()`), because it is the only account of WHY a decode died.
2. Take `child.stdout`; hand `child.stderr` to `drain_stderr`; return `Self { child, stdout, frame_bytes, stderr, drain, frames: 0 }`.

## drain_stderr

```rust
fn drain_stderr(pipe: ChildStderr, sink: Arc<Mutex<String>>) -> JoinHandle<()>
```

Reads ffmpeg's stderr line by line on its own thread into a rolling tail, clearing the buffer wholesale once it would exceed `TAIL_CAP` (2000 bytes) rather than slicing it - a byte slice could split a UTF-8 char boundary and panic.

**Why a thread rather than reading the pipe after EOF:** ffmpeg blocks once the ~64 KB stderr pipe fills, and a blocked ffmpeg stops writing *stdout* too - so a reader waiting for the next frame would deadlock against the very message it is waiting to read. A per-decoder thread costs a few tens of microseconds and exits by itself when the pipe hits EOF (process exit, or the `Drop` kill).

## classify_end

```rust
fn classify_end(success: bool, status: &str, frames: u64, tail: &str) -> Result<bool>
```

Decides what a closed decoder stdout MEANS, given ffmpeg's exit status. `Ok(false)` (clean end of stream) only when it exited 0; otherwise an `Err` reading `ffmpeg decode failed (<status>) after <frames> frame(s): <stderr tail>` (or `no stderr output` when ffmpeg said nothing).

**Why this exists:** ffmpeg closes stdout identically whether it finished the file or died on it - an unreadable/corrupt `video.mp4`, a filtergraph newer ffmpeg rejects, a missing codec. Mapping `UnexpectedEof` straight to `Ok(false)` therefore made a total decode failure byte-for-byte indistinguishable from a normal finish: the export loop fell back to a black frame for every frame, ran the progress bar 0->100%, muxed the audio, and emitted `export-done`. The user got a full-length, correctly-timed, entirely black `final.mp4` and no error anywhere, with ffmpeg's reason discarded at the `Stdio::null()`. Split out as a pure function so that classification is unit-testable without spawning a process.

### Behaviors worth knowing

- `a_clean_exit_is_end_of_stream` (unit test): `success = true` -> `Ok(false)`, whatever else is passed.
- `a_failed_exit_errors_with_the_stderr_tail` (unit test): a non-zero exit surfaces ffmpeg's own message and the frame count.
- `a_failed_exit_without_stderr_is_still_an_error` (unit test): silence is never promoted to a clean EOF.

## decode_args

```rust
fn decode_args(video: &Path, rate: f64, input_rate: bool, seek_ms: Option<u64>, crop: Option<(u32, u32)>, cover_scale: Option<(u32, u32)>, target_dims: Option<(u32, u32)>, pix_fmt: &str) -> Vec<String>
```

Pure builder for `RawDecoder::spawn`'s ffmpeg arg list - no process spawn, so the `-r`/`-vf`/`-pix_fmt` selection is unit-testable directly. Mirrors the exact arg order the command used to be built inline: `-v error -hwaccel auto`, optional `-ss seek_ms/1000.0`, optional input `-r rate` (`input_rate == true`), `-i video`, optional output `-r rate` (`input_rate == false`), `-sws_flags fast_bilinear`, then one `-vf` built from up to two parts joined by a comma: `crop=w:h:0:0` first when `crop` is set, then either `scale=w:h:flags=fast_bilinear` (`target_dims`) or `scale=W:H:force_original_aspect_ratio=increase,crop=W:H` (`cover_scale`); no `-vf` at all when none of the three is set. Then `-f rawvideo -pix_fmt <pix_fmt>`, `-` (stdout). NOTE: `flags` is a `scale` option, NOT a `crop` option - putting it on `crop` makes newer ffmpeg reject the whole filtergraph ("Option not found"), which silently zeroes the webcam decode and drops the camera from every export; the global `-sws_flags fast_bilinear` already covers the scale.

`-ss` lands BEFORE `-i`, which is what makes it an accurate seek that also resets the output timestamps, and it lands there whatever `input_rate` says, so a seeked decode at an OUTPUT rate reads `-ss t -i video -r rate`: ffmpeg drops everything before `t` and then resamples what is left onto the rate grid, so frame `n` off the pipe is the source frame `n` ticks after `t`. A `None` emits nothing at all, which is a different command from `-ss 0.0000` and the one the export's un-seeked screen decode has always issued. The caller is responsible for choosing a `t` the grid agrees with: `ScreenPipe::spawn`'s per-clip callers quantise it to the output frame period (`first_k * 1000 / out_fps`, `mod.md`) so the seek lands exactly on the frame the plan asks for rather than between two of them.

### Inputs

Same as `RawDecoder::spawn` minus `frame_bytes` (that field is stored on `RawDecoder`, not needed to build the ffmpeg args).

### Returns

`Vec<String>` - the full ffmpeg argument list (everything after the `ffmpeg` binary name itself).

### Behaviors

- `positive_output_rate_emits_r_after_i` - `rate > 0.0, input_rate = false` (the screen/webcam decode's shape) places `-r <rate>` AFTER `-i`, formatted `%.4f`.
- `positive_input_rate_emits_r_before_i` - `input_rate = true` places `-r` BEFORE `-i` instead.
- `non_positive_rate_omits_r_entirely` - `rate <= 0.0` (native-rate decode, e.g. the preview engine's seek-one-frame calls) emits no `-r` at all.
- `cover_scale_emits_the_boxs_own_w_h` - a non-square box (`Some((448, 252))`) emits `scale=448:252:force_original_aspect_ratio=increase,crop=448:252`. The box itself now comes from `render::meta::webcam_box` (the SOURCE video's aspect, one box for the whole export) rather than from any one panel; per-panel framing happens in the compositors.
- `cover_scale_of_equal_dims_is_the_old_square_filter` - `Some((420, 420))` emits the byte-identical filter the square-only version did.
- `crop_is_exact_and_comes_before_any_scale` - `crop: Some((1696, 954))` alone emits `crop=1696:954:0:0`; with `target_dims` too the crop comes first (`crop=1696:954:0:0,scale=848:477:flags=fast_bilinear`); with nothing set there is no `-vf`.
- `a_seek_goes_in_before_the_input_and_nothing_else_moves` - the screen decode's own shape is pinned as a whole argv, and `seek_ms: Some(4000)` splices `-ss 4.0000` in at index 4, between `-hwaccel auto` and `-i`, leaving every other argument where it was. The pin is what a per-clip respawn rests on, since a `-ss` that drifted after `-i` would stop being an accurate seek.

## RawDecoder::read_frame

```rust
pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool>
```

Reads exactly one frame into `buf` from the ffmpeg stdout pipe.

### Inputs

- `buf: &mut [u8]` - caller-owned buffer of exactly `frame_bytes`. *Why caller-owned:* avoids a per-frame allocation in the hot render loop.*

### Returns

`Result<bool>` - `Ok(true)` when a full frame was read (and `frames` is incremented); `Ok(false)` ONLY for a CLEAN end-of-stream; `Err` on any other I/O error, and on an end-of-stream that ffmpeg reached by failing.

### Implementation

1. `self.stdout.read_exact(buf)` -> `Ok(true)` on success, `Err` on a non-EOF I/O error.
2. On `UnexpectedEof`, hand off to `end_of_stream`: `child.wait()` reaps the process, the `drain` thread is joined (its pipe hit EOF the moment ffmpeg exited, so this returns at once and the tail is complete rather than racing the exit), and `classify_end` turns the exit status into `Ok(false)` or an `Err`.

## RawDecoder::end_of_stream

```rust
fn end_of_stream(&mut self) -> Result<bool>
```

Reap-and-classify half of `read_frame`, split out only so the read path stays a three-arm match. Waits on the child, joins the stderr drain, trims the tail, and returns `classify_end(status.success(), &status.to_string(), self.frames, &tail)`. Calling `wait()` here is safe alongside `Drop`'s own `kill()`/`wait()`: `std::process::Child` caches the exit status, so the second wait is a no-op.
