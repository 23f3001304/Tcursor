# src-tauri/src/export/pipeline/mod.rs

The 3-stage export pipeline that overlaps decode, composite, and encode. The screen and webcam decoders each run on their own thread (bodies in `pipeline_decode.rs`), reading frames into pooled buffers and streaming them over bounded channels; the exporter's composite loop pulls from those channels via `ScreenPipe`/`WebcamPipe` while the encoder thread drains the composited frames. The result is that decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1, so wall time trends from `sum(decode, composite)` toward `max(decode, composite)`.

The screen decodes 1:1 with output frames (`ScreenPipe::next`): the screen `RawDecoder` is spawned with `-r out_fps` (same as `WebcamPipe`), so ffmpeg itself rate-converts the decode to the export's output rate and output frame k is decoded frame k regardless of whether `out_fps` matches the source capture rate - there is no frame-selection or re-timing step (re-timing against `sync.json` was the old A/V-drift bug, since a fixed-rate re-time has more/fewer frames than the recorded delivered-frame timestamps). Before this, `ScreenPipe::spawn` passed a `0.0` rate (no `-r`, native source rate), which was only correct when `out_fps` happened to equal the capture rate - any other export fps played the decoded video at the wrong speed against real-time audio. The threads only move bytes between stages and never change them. Decode-thread errors are stored in a shared `Arc<Mutex<Option<Error>>>` and surfaced on the main side (via the pipe's `next` calls or `join`), which distinguishes a real decode failure from a clean EOF (channel closed with no stored error).

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

## audio_shift_ms

```rust
pub fn audio_shift_ms(track_ms: Option<u64>, video_start: u64, trim_in_q_ms: u64) -> i64
```

The mux shift (ms, negative = start the track later) for ONE audio track: where its own first sample sits relative to the exported video's frame 0.

### Inputs

- `track_ms: Option<u64>` - the track's own start timestamp on the capture clock (`Timeline.mic_ms` / `system_ms`), `None` when that track was not recorded. *Why an Option:* a missing track yields a `0` base rather than a special case at the call site; the trim term still applies so the two tracks stay consistent.
- `video_start: u64` - the first captured video frame's timestamp. *Why:* it is the origin the exported file's frame 0 is aligned to.
- `trim_in_q_ms: u64` - the FRAME-FLOORED trim-in, i.e. `k_in * 1000 / out_fps` where `k_in` comes from `trim_frame_bounds`. **Not** the raw `trim_in_ms`. *Why this is the whole point of the function:* `trim_frame_bounds` FLOORS the trim point to a frame index, so exported frame 0 shows the source content at `trim_in_q_ms`, which is `<= trim_in_ms`. Subtracting the unquantised `trim_in_ms` therefore advanced the audio further than the picture by up to a full frame - 16.7 ms at 60 fps, 33 ms at `Fps::F30` - as a constant lip-sync lead across the entire export.

### Returns

`i64` - `(track_ms - video_start) - trim_in_q_ms`, ready for `audio_mux::mux`'s `-itsoffset`. The mic call site adds `Settings.audio_offset_ms` (the user's manual nudge) on top.

### Behaviors worth knowing

- `audio_shift_uses_the_frame_floored_trim_in` (unit test): a 1234 ms trim-in at 30 fps floors to frame 37 = 1233 ms, and the shift uses 1233; a frame-aligned trim-in and the untrimmed case are both unchanged from the old formula.

## webcam_warning

```rust
pub fn webcam_warning(had_webcam: bool, fail: Option<&str>, frames: u64) -> Option<String>
```

How the webcam decode ended, as a user-facing warning string (`None` = nothing to say). `exporter::export` returns it and `run_export` emits it as an `export-warning`.

### Inputs

- `had_webcam: bool` - whether a `WebcamPipe` was spawned at all. *Why:* with no webcam file there is nothing to warn about, and the camera panel is legitimately absent.
- `fail: Option<&str>` - the first webcam decode error seen, from the composite loop OR from `WebcamPipe::join` after it. *Why not an `Err` return in the exporter:* the screen is the deliverable and the camera is one panel of it; by the time either error is known the file is fully composited and encoded, so failing the export would throw away good work for a partial defect.
- `frames: u64` - webcam frames actually decoded. *Why it is the deciding input:* it separates the two shapes of failure, which produce DIFFERENT files. With zero frames the camera panel is ABSENT for the whole export. With some frames, `exporter`'s `last_webcam` hold keeps rendering the last decoded frame, so the panel is present but FROZEN from that point - telling the user it was "exported without the camera panel" would send them looking for a missing panel that is on screen.

### Returns

`Option<String>`: `None` for no webcam and for a healthy decode; otherwise one of three messages - "no frames … without the camera panel", "failed before any frame … without the camera panel: <err>", or "failed after N frames - the camera panel is frozen from that point on: <err>".

### Behaviors worth knowing

- `webcam_warning_separates_zero_frames_from_a_partial_failure` (unit test): pins all five branches, including that the partial-failure message must NOT claim the panel is absent.

## ScreenPipe

```rust
pub struct ScreenPipe
```

Owns the screen decode thread's receiving end plus the last-delivered frame (`cur`). The thread streams `(buf, idx)` for every decoded frame; `next` returns them 1:1 with output frames, recycling each superseded buffer back into the decode pool and holding the last frame on EOF.

## ScreenPipe::spawn

```rust
pub fn spawn(video: &Path, screen_bytes: usize, target_dims: Option<(u32, u32)>, depth: usize, out_fps: u64) -> Result<ScreenPipe>
```

Spawns the screen `RawDecoder` at `out_fps` (no seek, `nv12` pixel format, optional `target_dims` scale) and its decode thread. The decoder is created here rather than inside the thread so spawn errors surface immediately to the caller. Passing `out_fps` (rather than a `0.0`/native rate, like `WebcamPipe::spawn` already did) rate-converts the decode to the export's output rate, so the composite loop's 1:1 pull stays correct even when `out_fps` differs from the capture rate.

### Inputs

- `video: &Path` - the screen recording. *Why:* the primary decode input.*
- `screen_bytes: usize` - bytes per screen frame (nv12: `sw*sh` Y + `sw*sh/2` UV). *Why:* sizes both the pooled decode buffers and the decoder's `read_frame` assertion.*
- `target_dims: Option<(u32, u32)>` - optional scale target passed to the decoder (`None` = native size). *Why:* lets a caller decode straight to a smaller working size.*
- `depth: usize` - sizes both the bounded channel and the recycled buffer pool. *Why:* provides backpressure so the decode thread stays a bounded number of frames ahead.*
- `out_fps: u64` - the export's resolved output frame rate, passed to the decoder as `-r out_fps`. *Why:* forces ffmpeg to rate-convert the decode to the export's output rate rather than the source capture rate, so `next`'s 1:1-with-output-frames pull stays correct at any export fps (matches `WebcamPipe::spawn`'s existing `out_fps` parameter).*

### Returns

`Result<ScreenPipe>` - the running pipe, or the decoder spawn error.

## ScreenPipe::next

```rust
pub fn next(&mut self) -> Result<Option<&[u8]>>
```

Returns the next decoded screen frame - 1:1 with output frames, blocking on the decode channel as needed. `video.mp4` is decoded at `-r out_fps` (see `spawn`), so output frame k IS decoded frame k at any export rate; there is no `sync.json` re-timing (that was the A/V-drift bug - a fixed-rate re-time has more/fewer frames than the recorded delivered-frame timestamps, which skewed the video against the real-time audio). On EOF it holds the last decoded frame (matches the old clamp). `Ok(None)` occurs only for a zero-frame video. A stored decode error is returned as `Err`; the superseded buffer is recycled.

### Returns

`Result<Option<&[u8]>>` - the decoded frame's nv12 bytes (`Some`), or `None` for a zero-frame video.

## ScreenPipe::held

```rust
pub fn held(&self) -> Option<&[u8]>
```

The frame `next` last delivered (held through EOF). The frame loop composites from it, so an output frame that re-uses the previous recording frame (slow motion) or that follows several skipped decodes (a cut, a fast span) never calls `next` twice for one frame.

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
pub fn spawn(webcam: &Path, video_start: u64, dims: (u32, u32), wc_bytes: usize, depth: usize, out_fps: u64) -> Result<WebcamPipe>
```

Spawns the webcam `RawDecoder` (at `out_fps`, seeked to `video_start`, cover-cropped to `dims`) and its decode thread. Like `ScreenPipe::spawn`, spawn errors surface immediately.

### Inputs

- `webcam: &Path` - the webcam recording. *Why:* the decode input for the picture-in-picture panel.*
- `video_start: u64` - ms offset to seek the webcam to, aligning it with the screen timeline. *Why:* the two streams start at different wall-clock offsets.*
- `dims: (u32, u32)` - the `(w, h)` box the webcam is cover-cropped to (`RenderMeta::webcam_w`/`webcam_h`, from `render::meta::webcam_box`). *Why a pair, not a square side:* it is ONE box for the whole export carrying the SOURCE video's own aspect, which the compositors then cover-crop to each panel's aspect per frame - decoding at any single PANEL's aspect (or at a square) discards pixels that a differently-shaped panel later in the layout track still needs. Held on the pipe (not resent per frame) since it is fixed for the whole export.*
- `wc_bytes: usize` - bytes per webcam frame (`dims.0 * dims.1 * 4`). *Why:* sizes the pooled buffers and the decoder assertion.*
- `depth: usize` - channel + pool depth, as for `ScreenPipe`.
- `out_fps: u64` - the export's resolved output frame rate (`exporter::export`'s own `out_fps`, from `ExportSettings.fps`). *Why threaded in rather than using the `OUT_FPS` constant:* the webcam decode cadence must match whatever rate the composite loop and encoder actually run at, not always exactly 60 - previously this was hardcoded to the `OUT_FPS` constant regardless of the (formerly fixed) export rate.

### Returns

`Result<WebcamPipe>` - the running pipe, or the decoder spawn error.

## WebcamPipe::next

```rust
pub fn next(&mut self) -> Result<Option<(Vec<u8>, u32, u32)>>
```

Returns the next webcam frame `(buf, w, h)` (the `dims` `spawn` was given), blocking on the decode channel. The decode thread streams bare buffers; the dims are attached here from the pipe's own state. `Ok(None)` at EOF matches the old `read_webcam` dropping the decoder so every later frame has no camera. A stored decode error is returned as `Err`. The caller returns `buf` to the pool via `recycle` once it has been composited.

### Returns

`Result<Option<(Vec<u8>, u32, u32)>>` - `Some((bgra, w, h))` while frames remain; `None` at EOF.

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

## plan_walk

`PlanCursor`: the pure bookkeeping of feeding sequential decoders along a `TimeMap::frame_plan` (how many recording frames to decode before each output frame, 0 to re-use the held one).

## bg_pipe

The VIDEO/GIF background's decode stream - a third stream on the same output clock, `WebcamPipe` being the template (it even reuses `spawn_webcam` as its thread body). Key items: `bg_decode_args` (the pure `-stream_loop -1 -an -r out_fps` arg list), `BgPipe::open`/`feed`/`join`. Frames are dimmed on their way out and swapped straight into `FrameRenderer`'s `bg` buffer.

## run

Thin Tauri command adapter that launches the export on a background thread and bridges results to the frontend as events. Key items: `run_export(app, folder, settings)` - fire-and-forget; emits `export-progress`, `export-done`, and `export-error`.

## ffio

FFmpeg and ffprobe spawn helpers, raw BGRA frame reader, and bundled-image decode/crop utilities. Key items: `RawDecoder` (spawned ffmpeg subprocess, `spawn` + `read_frame`, arg list built by the pure `decode_args`), `probe_dims`, `probe_duration`, `probe_frame_count`, `decode_image`, `decode_cursor`, `crop_to_alpha`, `png_dims`.

## audio_segments

The time remap's kept ranges as one ffmpeg filter chain (`atrim` + `asetpts` + `atempo` per segment, `concat`), inserted by `audio_mux::mux_args`; the identity emits nothing.

## silence

Remove silences: `silencedetect` over the recorded tracks, intersected, shifted onto the clip clock, padded, filtered, clamped; the `detect_silences` command the editor's button calls.

## audio_mux

Muxes the encoded silent video with microphone and/or system audio into `final.<ext>` (`<ext>` from the export's `Format`), handling all four audio combinations and applying per-track A/V sync offsets and volume gain. Key items: `mux(tmp, paths, format, mic_shift_ms, sys_shift_ms, mic_vol, sys_vol) -> Result<()>`.

## timeline

Builds the per-frame timestamp vector from the sync log or a synthesized uniform fallback, plus audio track start offsets. Key items: `Timeline` struct (`frames: Vec<u64>`, `events_ms`, `mic_ms`, `system_ms`), `build_timeline(paths, log, fps) -> Timeline`.
