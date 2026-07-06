# src-tauri/src/export/pipeline/mod.rs

The 3-stage export pipeline that overlaps decode, composite, and encode. The screen and webcam decoders each run on their own thread (bodies in `pipeline_decode.rs`), reading frames into pooled buffers and streaming them over bounded channels; the exporter's composite loop pulls from those channels via `ScreenPipe`/`WebcamPipe` while the encoder thread drains the composited frames. The result is that decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1, so wall time trends from `sum(decode, composite)` toward `max(decode, composite)`.

The frame-selection logic is factored into the pure `advance_index` function so it can be unit-tested in isolation; the threads only move bytes between stages and never change them, keeping output byte-identical to the old sequential loop. Decode-thread errors are stored in a shared `Arc<Mutex<Option<Error>>>` and surfaced on the main side (via the pipe's `next_*` calls or `join`), which distinguishes a real decode failure from a clean EOF (channel closed with no stored error).

## advance_index

```rust
pub fn advance_index(frames: &[u64], cur: usize, t: u64) -> usize
```

Returns the index of the captured frame active at output time `t`: the last `i >= cur` with `frames[i] <= t`, clamped to the last frame. This is the exporter's variable-frame-rate advance rule (`while frames[i+1] <= t`) lifted verbatim and made pure, so it drives `ScreenPipe::next_at` and is covered by a unit test.

### Inputs

- `frames: &[u64]` - captured-frame timestamps (ms), assumed non-decreasing. *Why:* the source video is variable-frame-rate; each output tick maps to whichever captured frame was live at that time.*
- `cur: usize` - the currently selected index; the search only ever moves forward from here. *Why:* the exporter time base is monotonic, so the decoder never rewinds.*
- `t: u64` - output time in ms.

### Returns

`usize` - the selected captured-frame index, `>= cur` and `< frames.len()` (clamped to the last frame; never past the end).

### Behaviors worth knowing

- `advance_index_matches_sequential_selection` - at `t == frames[i]` the boundary selects `i` (uses `<=`, not `<`); a `t` far past the end clamps at `frames.len()-1`; it never returns less than `cur`.

## ScreenPipe

```rust
pub struct ScreenPipe
```

Owns the screen decode thread's receiving end plus the main-side VFR advance state. The thread streams `(buf, idx)` for every decoded captured frame; `next_at` supersedes the current frame toward the requested output time, recycling each passed-over buffer back into the decode pool.

## ScreenPipe::spawn

```rust
pub fn spawn(video: &Path, screen_bytes: usize, frames: Vec<u64>, depth: usize) -> Result<ScreenPipe>
```

Spawns the screen `RawDecoder` (native rate, no seek/scale) and its decode thread. The decoder is created here rather than inside the thread so spawn errors surface immediately to the caller.

### Inputs

- `video: &Path` - the screen recording. *Why:* the primary decode input.*
- `screen_bytes: usize` - bytes per screen frame (`sw * sh * 4`). *Why:* sizes both the pooled decode buffers and the decoder's `read_frame` assertion.*
- `frames: Vec<u64>` - captured-frame timestamps for `advance_index`. *Why owned:* the pipe outlives the caller's borrow of `meta.tl.frames` and is queried every output tick.*
- `depth: usize` - sizes both the bounded channel and the recycled buffer pool. *Why:* provides backpressure so the decode thread stays a bounded number of frames ahead.*

### Returns

`Result<ScreenPipe>` - the running pipe, or the decoder spawn error.

## ScreenPipe::next_at

```rust
pub fn next_at(&mut self, t: u64) -> Result<Option<&[u8]>>
```

Returns the screen frame active at output time `t`, blocking on the decode channel as needed. Lazily pulls the first frame on the initial call, then advances via `advance_index`, recycling each superseded buffer. `Ok(None)` occurs only for a zero-frame video (matches the old all-zeros `screen_buf` when the first read hit EOF). On EOF short of the target it clamps at the last decoded frame (the old `have == false` behavior). A stored decode error is returned as `Err`.

### Returns

`Result<Option<&[u8]>>` - the active frame's BGRA bytes (`Some`), or `None` for a zero-frame video.

## ScreenPipe::join

```rust
pub fn join(self) -> Result<()>
```

Joins the decode thread and surfaces any stored decode error (even one the loop never observed because it never advanced that far). Drops the receiver first so a decode thread still blocked trying to send (its decoder produced more frames than the loop consumed) is released and the join cannot hang.

## WebcamPipe

```rust
pub struct WebcamPipe
```

The webcam decode thread's receiving end. The webcam decoder runs at `OUT_FPS` (1:1 with output frames, no superseding), so this is simpler than `ScreenPipe`: one frame per output tick, recycled by the caller after compositing.

## WebcamPipe::spawn

```rust
pub fn spawn(webcam: &Path, video_start: u64, size: u32, wc_bytes: usize, depth: usize) -> Result<WebcamPipe>
```

Spawns the webcam `RawDecoder` (at `OUT_FPS`, seeked to `video_start`, cover-cropped to a `size` square) and its decode thread. Like `ScreenPipe::spawn`, spawn errors surface immediately.

### Inputs

- `webcam: &Path` - the webcam recording. *Why:* the decode input for the picture-in-picture panel.*
- `video_start: u64` - ms offset to seek the webcam to, aligning it with the screen timeline. *Why:* the two streams start at different wall-clock offsets.*
- `size: u32` - square side length the webcam is cover-cropped to. *Why:* the compositor expects a square camera panel.*
- `wc_bytes: usize` - bytes per webcam frame (`size * size * 4`). *Why:* sizes the pooled buffers and the decoder assertion.*
- `depth: usize` - channel + pool depth, as for `ScreenPipe`.

### Returns

`Result<WebcamPipe>` - the running pipe, or the decoder spawn error.

## WebcamPipe::next

```rust
pub fn next(&mut self) -> Result<Option<(Vec<u8>, u32, u32)>>
```

Returns the next webcam frame `(buf, w, h)` (both dims equal `size`), blocking on the decode channel. `Ok(None)` at EOF matches the old `read_webcam` dropping the decoder so every later frame has no camera. A stored decode error is returned as `Err`. The caller returns `buf` to the pool via `recycle` once it has been composited.

### Returns

`Result<Option<(Vec<u8>, u32, u32)>>` - `Some((bgra, size, size))` while frames remain; `None` at EOF.

## WebcamPipe::recycle

```rust
pub fn recycle(&self, buf: Vec<u8>)
```

Returns a composited webcam buffer to the decode pool for reuse (silently dropped if the pool is gone). Called after `composite_at` has consumed the frame, so the ~1 MB webcam buffer is recycled rather than freed each tick.

## WebcamPipe::join

```rust
pub fn join(self) -> Result<()>
```

Joins the decode thread and surfaces any stored decode error, dropping the receiver first so an over-running webcam decoder (webcam longer than the output) cannot hang the join. Same pattern as `ScreenPipe::join`.

## exporter

Top-level export orchestrator: calls `FrameRenderer::new`, spawns the screen/webcam decode threads (`ScreenPipe`/`WebcamPipe`) and the encoder thread, then drives the per-frame composite loop via `step_camera` + `composite_at` and muxes audio. Key items: `export(paths, fps, on_progress) -> Result<()>` - single public entry point.

## run

Thin Tauri command adapter that launches the export on a background thread and bridges results to the frontend as events. Key items: `run_export(app, folder)` - fire-and-forget; emits `export-progress`, `export-done`, and `export-error`.

## ffio

FFmpeg and ffprobe spawn helpers, raw BGRA frame reader, and bundled-image decode/crop utilities. Key items: `RawDecoder` (spawned ffmpeg subprocess, `spawn` + `read_frame`), `probe_dims`, `probe_duration`, `probe_frame_count`, `decode_image`, `decode_cursor`, `crop_to_alpha`, `png_dims`.

## audio_mux

Muxes the encoded silent video with microphone and/or system audio into `final.mp4`, handling all four audio combinations and applying per-track A/V sync offsets. Key items: `mux(tmp, paths, mic_shift_ms, sys_shift_ms) -> Result<()>`.

## timeline

Builds the per-frame timestamp vector from the sync log or a synthesized uniform fallback, plus audio track start offsets. Key items: `Timeline` struct (`frames: Vec<u64>`, `events_ms`, `mic_ms`, `system_ms`), `build_timeline(paths, log, fps) -> Timeline`.
