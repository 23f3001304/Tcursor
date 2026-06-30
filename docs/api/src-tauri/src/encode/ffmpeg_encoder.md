# src-tauri/src/encode/ffmpeg_encoder.rs

Implements `FrameSink` by piping raw BGRA frames to a child `ffmpeg` process that encodes them as H.264 MP4. The key property is hardware-first encoder selection: a one-time probe at startup picks `h264_nvenc`, `h264_qsv`, `h264_amf`, or `h264_mf` (in that order) before falling back to software `libx264`. The probe result is cached in a `OnceLock` so every subsequent recording pays zero probe latency.

## FfmpegFrameSink

```rust
pub struct FfmpegFrameSink {
    child: Child,
    width: u32,
    height: u32,
}
```

Active encode session. Holds the spawned ffmpeg process and the expected frame dimensions.

- `child: Child` - *Spawned ffmpeg process. `stdin` is a `Stdio::piped()` handle to which `push` writes BGRA bytes. Closing `stdin` in `finish` signals EOF so ffmpeg flushes and exits.*
- `width: u32` - *Expected frame width. Used in `debug_assert` inside `push` to catch dimension mismatches early in development builds.*
- `height: u32` - *Expected frame height; same role.*

### Used by

- `session/recorder.rs` - constructs `FfmpegFrameSink::new_vfr` for normal recording (and `new` for game mode) and boxes it as `dyn FrameSink` for the recording session.
- `export/exporter.rs` - constructs `FfmpegFrameSink::new_hq` for the offline export path.

## prewarm

```rust
pub fn prewarm()
```

Forces the hardware encoder probe to run on the calling thread so the result is cached before the first recording. Also confirms the ffmpeg binary launches, and appends a one-line diagnostic to `%TEMP%/tcursor-ffmpeg.log` with `encoder=<name> ffmpeg_runs=<Ok/Err>`. If the log file cannot be opened, the diagnostic is silently skipped. Subsequent calls are free because `OnceLock::get_or_init` returns the cached value immediately.

### Implementation

1. Call `h264_encoder()`, which runs `probe_encoder` for each candidate in order. *Why on a background thread (as the caller in `lib.rs` does):* the first ffmpeg launch takes ~100 ms while the OS scans the bundled binary; running it at startup hides that latency behind app boot so the first recording's `FfmpegFrameSink::new` is fast and audio capture is not delayed.*
2. Run `ffmpeg -version` via `ffcmd`. *Why separately:* `probe_encoder` uses `ffmpeg -f lavfi ...` which may succeed or fail silently if the binary is blocked by AV software; `-version` is a simpler liveness check.*
3. Append diagnostic to `%TEMP%/tcursor-ffmpeg.log`.

### Returns

`()` - all errors are logged or silently swallowed; `prewarm` is best-effort.

## FfmpegFrameSink::new

```rust
pub fn new(out_path: &str, width: u32, height: u32, fps: u32) -> std::io::Result<Self>
```

Spawns ffmpeg for **game-mode** recording at a fixed CFR `fps` (`-framerate`). Normal recording uses `new_vfr` instead - a fixed rate is correct only when the source is paced to it (game mode's `run_paced`); for a free-running capture it would mislabel the timing and the file would play sped up.

### Inputs

- `out_path: &str` - Destination MP4 file path. *Why:* written directly by ffmpeg; no intermediate temp file.*
- `width: u32`, `height: u32` - Frame dimensions. *Why:* passed to ffmpeg as `-video_size {width}x{height}` so it knows how to stride the incoming raw bytes.*
- `fps: u32` - Target frame rate. *Why:* passed as `-framerate {fps:.4}` so ffmpeg embeds the correct display rate; also sets the PTS increment for each frame.*

### Returns

`std::io::Result<Self>` - fails if ffmpeg cannot be spawned (binary not found, permission denied, or I/O error opening stdin).

## FfmpegFrameSink::new_vfr

```rust
pub fn new_vfr(out_path: &str, width: u32, height: u32) -> std::io::Result<Self>
```

Spawns ffmpeg for **normal real-time recording in VFR mode**: each frame is stamped with its real arrival (wall-clock) time instead of a fixed rate, so `video.mp4` plays at true speed even when the capture rate dips below the display refresh.

### Inputs

- `out_path: &str`, `width: u32`, `height: u32` - Same roles as in `new`. No `fps` argument: there is no fixed rate.

### Implementation

Drops `-framerate` and instead passes `-use_wallclock_as_timestamps 1` on the rawvideo input (ffmpeg stamps each frame with the system clock as it reads it from the pipe - and the record loop pushes frames in real time) plus `-fps_mode passthrough` on the output (keep *every* frame with its timestamp, never dropping, so the frame↔`sync.json` index map stays 1:1 and the export - which times frames by `sync.json`, not the file's PTS - is unaffected).

### Returns

`std::io::Result<Self>` - same as `new`.

## FfmpegFrameSink::new_hq

```rust
pub fn new_hq(out_path: &str, width: u32, height: u32, fps: f64) -> std::io::Result<Self>
```

Spawns ffmpeg for high-quality offline export.

### Inputs

- `out_path: &str`, `width: u32`, `height: u32` - Same roles as in `new`.
- `fps: f64` - *`f64` instead of `u32` so the exporter can correct a mislabeled source frame rate (e.g. 29.97 vs 30) without precision loss. Formatted to 4 decimal places in the ffmpeg command.*

### Returns

`std::io::Result<Self>` - same as `new`.

## FfmpegFrameSink::push

```rust
fn push(&mut self, f: &Frame) -> std::io::Result<()>
```

Writes all bytes of `f.bgra` to ffmpeg's stdin in one `write_all` call.

### Inputs

- `f: &Frame` - The frame to encode. *Why `&Frame` rather than owned:* the caller (encoder loop) may need to keep the frame for other purposes; the sink only needs to read the bytes.*

### Implementation

1. In debug builds, `debug_assert_eq!((f.width, f.height), (self.width, self.height))`. *Why assert:* a dimension mismatch produces corrupted output silently in release builds; the assert catches it in development.*
2. Get a mutable reference to `child.stdin` via `.as_mut().expect(...)`. *Why `expect`:* `stdin` is `Some` from construction until `finish` takes it; panicking here indicates a programming error.*
3. Call `stdin.write_all(&f.bgra)`.

### Returns

`std::io::Result<()>` - fails if ffmpeg exited early and the stdin pipe is broken.

## FfmpegFrameSink::finish

```rust
fn finish(mut self: Box<Self>) -> std::io::Result<()>
```

Closes stdin to signal EOF to ffmpeg, then waits for ffmpeg to finish encoding and exit.

### Implementation

1. `drop(self.child.stdin.take())` - closes the stdin pipe. *Why drop explicitly:* ffmpeg exits its encoding loop only when stdin reaches EOF; leaving stdin open would cause `wait()` to block forever.*
2. `self.child.wait()` - blocks until ffmpeg exits. *Why block:* the encoder thread must not return control to the recording session until the MP4 is fully written.*
3. If `status.success()` is false, return `Err` with the exit status in the message.

### Returns

`std::io::Result<()>` - fails if `wait()` returns an OS error, or if ffmpeg exits with a non-zero status (e.g. encoding error, codec not found).
