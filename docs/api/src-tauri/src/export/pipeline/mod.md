# src-tauri/src/export/pipeline/mod.rs

The 3-stage export pipeline that overlaps decode, composite, and encode. The screen and webcam decoders each run on their own thread (bodies in `pipeline_decode.rs`), reading frames into pooled buffers and streaming them over bounded channels; the exporter's composite loop pulls from those channels via `ScreenPipe`/`WebcamPipe` while the encoder thread drains the composited frames. The result is that decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1, so wall time trends from `sum(decode, composite)` toward `max(decode, composite)`.

The screen decodes 1:1 with output frames (`ScreenPipe::next`): `video.mp4` is already CFR-60 real-time, so output frame k is decoded frame k - there is no frame-selection or re-timing step (re-timing against `sync.json` was the old A/V-drift bug, since the CFR encode has more frames than the recorded delivered-frame timestamps). The threads only move bytes between stages and never change them. Decode-thread errors are stored in a shared `Arc<Mutex<Option<Error>>>` and surfaced on the main side (via the pipe's `next` calls or `join`), which distinguishes a real decode failure from a clean EOF (channel closed with no stored error).

## trim_frame_bounds

```rust
pub fn trim_frame_bounds(trim_in_ms: u32, trim_out_ms: u32, out_fps: u64) -> (u64, u64)
```

Converts a resolved trim range (ms, from `Trim::resolve`) to INCLUSIVE output-frame index bounds `[k_in, k_last]` at `out_fps`, using the same `(ms * out_fps) / 1000` formula the exporter's own untrimmed loop bound always used - so an untrimmed clip (`trim_out_ms == full_dur_ms`) yields `k_last` identical to the old bound and composites the exact same frame count (back-compat).

### Inputs

- `trim_in_ms: u32`, `trim_out_ms: u32` - the resolved (not raw) trim range from `Trim::resolve`.
- `out_fps: u64` - the export frame rate.

### Returns

`(u64, u64)` - `(k_in, k_last)`, both inclusive. `k_last` is always `>= k_in`, so a degenerate (zero-length) trim still emits at least one frame rather than producing an empty export.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - gates which frames the composite loop actually renders/encodes.

### Behaviors

- `trim_frame_bounds_converts_ms_to_inclusive_frame_indices` - a `[2000, 8000]`ms trim at 60fps yields `(120, 480)`.
- `trim_frame_bounds_never_collapses_to_empty` - an equal in/out still yields `k_last >= k_in`.
- `untrimmed_last_index_matches_the_old_total_out_formula` - `trim_frame_bounds(0, full_dur_ms, fps)` reproduces the pre-trim `total_out = dur*fps/1000` bound exactly.

## ScreenPipe

```rust
pub struct ScreenPipe
```

Owns the screen decode thread's receiving end plus the last-delivered frame (`cur`). The thread streams `(buf, idx)` for every decoded frame; `next` returns them 1:1 with output frames, recycling each superseded buffer back into the decode pool and holding the last frame on EOF.

## ScreenPipe::spawn

```rust
pub fn spawn(video: &Path, screen_bytes: usize, target_dims: Option<(u32, u32)>, depth: usize) -> Result<ScreenPipe>
```

Spawns the screen `RawDecoder` (native rate, no seek, `nv12` pixel format, optional `target_dims` scale) and its decode thread. The decoder is created here rather than inside the thread so spawn errors surface immediately to the caller.

### Inputs

- `video: &Path` - the screen recording. *Why:* the primary decode input.*
- `screen_bytes: usize` - bytes per screen frame (nv12: `sw*sh` Y + `sw*sh/2` UV). *Why:* sizes both the pooled decode buffers and the decoder's `read_frame` assertion.*
- `target_dims: Option<(u32, u32)>` - optional scale target passed to the decoder (`None` = native size). *Why:* lets a caller decode straight to a smaller working size.*
- `depth: usize` - sizes both the bounded channel and the recycled buffer pool. *Why:* provides backpressure so the decode thread stays a bounded number of frames ahead.*

### Returns

`Result<ScreenPipe>` - the running pipe, or the decoder spawn error.

## ScreenPipe::next

```rust
pub fn next(&mut self) -> Result<Option<&[u8]>>
```

Returns the next decoded screen frame - 1:1 with output frames, blocking on the decode channel as needed. `video.mp4` is CFR-60 real-time, so output frame k IS decoded frame k; there is no `sync.json` re-timing (that was the A/V-drift bug - the CFR encode has more frames than the recorded delivered-frame timestamps, which skewed the video against the real-time audio). On EOF it holds the last decoded frame (matches the old clamp). `Ok(None)` occurs only for a zero-frame video. A stored decode error is returned as `Err`; the superseded buffer is recycled.

### Returns

`Result<Option<&[u8]>>` - the decoded frame's nv12 bytes (`Some`), or `None` for a zero-frame video.

## ScreenPipe::join

```rust
pub fn join(self) -> Result<()>
```

Joins the decode thread and surfaces any stored decode error (even one the loop never observed because it never advanced that far). Drops the receiver first so a decode thread still blocked trying to send (its decoder produced more frames than the loop consumed) is released and the join cannot hang.

## WebcamPipe

```rust
pub struct WebcamPipe
```

The webcam decode thread's receiving end. The webcam decoder runs at the export's resolved output frame rate (`out_fps`, from `ExportSettings.fps` via `Fps::resolve_hz` - 1:1 with output frames, no superseding), so this is simpler than `ScreenPipe`: one frame per output tick, recycled by the caller after compositing.

## WebcamPipe::spawn

```rust
pub fn spawn(webcam: &Path, video_start: u64, size: u32, wc_bytes: usize, depth: usize, out_fps: u64) -> Result<WebcamPipe>
```

Spawns the webcam `RawDecoder` (at `out_fps`, seeked to `video_start`, cover-cropped to a `size` square) and its decode thread. Like `ScreenPipe::spawn`, spawn errors surface immediately.

### Inputs

- `webcam: &Path` - the webcam recording. *Why:* the decode input for the picture-in-picture panel.*
- `video_start: u64` - ms offset to seek the webcam to, aligning it with the screen timeline. *Why:* the two streams start at different wall-clock offsets.*
- `size: u32` - square side length the webcam is cover-cropped to. *Why:* the compositor expects a square camera panel.*
- `wc_bytes: usize` - bytes per webcam frame (`size * size * 4`). *Why:* sizes the pooled buffers and the decoder assertion.*
- `depth: usize` - channel + pool depth, as for `ScreenPipe`.
- `out_fps: u64` - the export's resolved output frame rate (`exporter::export`'s own `out_fps`, from `ExportSettings.fps`). *Why threaded in rather than using the `OUT_FPS` constant:* the webcam decode cadence must match whatever rate the composite loop and encoder actually run at, not always exactly 60 - previously this was hardcoded to the `OUT_FPS` constant regardless of the (formerly fixed) export rate.

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

Top-level export orchestrator: calls `FrameRenderer::new`, spawns the screen/webcam decode threads (`ScreenPipe`/`WebcamPipe`) and the encoder thread, then drives the per-frame composite loop via `step_camera` + `composite_at` and muxes audio. Key items: `export(paths, settings, on_progress) -> Result<()>` - single public entry point; `settings: ExportSettings` resolves output resolution/fps/quality/format.

## run

Thin Tauri command adapter that launches the export on a background thread and bridges results to the frontend as events. Key items: `run_export(app, folder, settings)` - fire-and-forget; emits `export-progress`, `export-done`, and `export-error`.

## ffio

FFmpeg and ffprobe spawn helpers, raw BGRA frame reader, and bundled-image decode/crop utilities. Key items: `RawDecoder` (spawned ffmpeg subprocess, `spawn` + `read_frame`), `probe_dims`, `probe_duration`, `probe_frame_count`, `decode_image`, `decode_cursor`, `crop_to_alpha`, `png_dims`.

## audio_mux

Muxes the encoded silent video with microphone and/or system audio into `final.<ext>` (`<ext>` from the export's `Format`), handling all four audio combinations and applying per-track A/V sync offsets and volume gain. Key items: `mux(tmp, paths, format, mic_shift_ms, sys_shift_ms, mic_vol, sys_vol) -> Result<()>`.

## timeline

Builds the per-frame timestamp vector from the sync log or a synthesized uniform fallback, plus audio track start offsets. Key items: `Timeline` struct (`frames: Vec<u64>`, `events_ms`, `mic_ms`, `system_ms`), `build_timeline(paths, log, fps) -> Timeline`.
