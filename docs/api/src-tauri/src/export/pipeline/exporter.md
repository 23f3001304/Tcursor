# src-tauri/src/export/pipeline/exporter.rs

Top-level export orchestrator and the **composite stage** of the 3-stage decode -> composite -> encode pipeline. It loads a `FrameRenderer` from `render.rs` (which owns all per-frame compositing state), starts the screen/webcam decode threads via `ScreenPipe`/`WebcamPipe` (`pipeline.rs`), spawns the encoder thread, drives the per-frame composite loop, and muxes audio into `final.mp4`. All per-frame render logic (camera sim, compositor, FX overlays, cursor) lives in `render::FrameRenderer`; the decode threads and the encoder each run concurrently, connected by bounded channels with recycled buffer pools (`pool.rs`), so decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1. Output is byte-identical to the old sequential loop - only the overlap and buffer recycling changed.

## export

```rust
pub fn export(paths: &ProjectPaths, settings: ExportSettings, on_progress: impl Fn(u8)) -> Result<()>
```

Renders the recording at `paths` into `paths.folder/final.<ext>` (`<ext>` from `settings.format`).

### Inputs

- `paths: &ProjectPaths` - project folder root; all asset paths are derived from it (`events.json`, `actions.json`, `video.raw`, optional `webcam.raw`, mic/system audio, `sync.json`, `edit.json`, `cursor.json`). *Why a single struct:* avoids threading individual file paths through every subsystem call.
- `settings: ExportSettings` (`export::settings::ExportSettings`) - the user's chosen resolution, fps, quality (CRF), and container format. *Why one struct:* every downstream consumer (`Layout`, the encode loop, `FfmpegFrameSink`, `audio_mux::mux`) needs a different slice of it, so passing the whole struct once avoids re-deriving values. `ExportSettings::default()` reproduces today's export exactly (Source resolution, 60fps, CRF 24, MP4/H.264) - see the back-compat tests in `settings.rs`/`ffmpeg_args.rs`.
- `on_progress: impl Fn(u8)` - callback receiving 0..=100 each time the percentage advances. *Why a callback:* the Tauri layer (in `run.rs`) wires this to `app.emit("export-progress", p)`; keeping it external lets the export logic stay testable without a live app handle.

### Returns

`Result<()>`. On success `final.<ext>` exists at `paths.folder`. On error the raw `anyhow::Error` propagates to the caller (including any decode-thread error surfaced by `ScreenPipe`/`WebcamPipe`).

### Implementation

1. Compute `capture_fps = primary_refresh_hz().min(60)` - the same display-refresh-derived value the old unconditional `fps` parameter used, now serving two roles: `FrameRenderer::new`'s capture-rate fallback (unchanged meaning, `build_timeline`'s last-resort denominator) and `settings.fps.resolve_hz(capture_fps)`'s own fallback for `Fps::Source` (so `Source` reproduces the old formula exactly without a second display query). `out_fps = settings.fps.resolve_hz(capture_fps)` is the export's actual output frame rate (`F30`/`F60` are fixed regardless of the display; `F60` is the default).
2. Call `FrameRenderer::new(paths, Layout::default(), capture_fps, settings.resolution, None)` to get `(renderer, meta)` (`None` = no preview downscale - a full export build). The renderer owns the event log, settings, compositor, FX renderer, camera sim, cursor state, and background. `meta` carries everything needed to drive the loop (`screen_bytes`, `webcam_size`, `video_start`, `video_end`, `tl`, `out_w`, `out_h`, `audio_offset_ms`, `trim`, `mic_volume`, `sys_volume`).
3. Open a `FfmpegFrameSink` writing to `tmp_export.<ext>` (`<ext>` from `settings.format`) via `new_medium(tmp_str, out_w, out_h, out_fps as f64, settings.format, settings.quality_crf)`. Build an `out_pool` (`BufPool`, `depth` output buffers) and spawn an encoder thread draining a `sync_channel(4)`; after pushing each `Frame` the encoder recycles its ~33 MB `bgra` buffer back into `out_pool`.
4. Start the decode threads: `ScreenPipe::spawn` (screen video) and, if `paths.webcam()` exists, `WebcamPipe::spawn(..., out_fps)` (webcam, at the resolved output rate). Spawn errors surface here. Allocate the `empty` zero-frame fallback as a BLACK nv12 buffer (Y=16, U=V=128) - NOT a zeroed buffer, which would decode to a green tint through the color convert - used only if the screen decode has produced nothing yet.
5. Resolve the trim gate: `full_dur_ms = meta.video_end - meta.video_start`; `meta.trim.resolve(full_dur_ms)` gives `(trim_in_ms, trim_out_ms)` (`out_ms == 0` = whole clip); `trim_frame_bounds` converts that to INCLUSIVE frame indices `(k_in, k_last)` at `out_fps`. An untrimmed clip yields `k_last` identical to the old unconditional bound, so nothing changes when there is no trim (back-compat).
6. Composite loop `0..=k_full_last` (the UNTRIMMED clip's own last index, so decode/camera-sim continuity is unaffected by trim): `spipe.next()` pulls the next decoded screen frame 1:1 with output frames - `video.mp4` is CFR-60 real-time, so output frame k IS decoded frame k, with no re-timing against `sync.json` - falling back to the black `empty` buffer only if the screen decode hasn't produced anything yet; `wpipe.next()` yields one webcam frame, or `None` at EOF, in which case the loop keeps compositing the LAST decoded webcam frame (held in `last_webcam`, recycled through the pool once superseded) instead of letting the PiP vanish early when the webcam stream is shorter than the screen; `renderer.step_camera(t)` always advances (even outside the trim range, for continuity) - then `k < k_in` `continue`s (decoded/stepped but not composited) and `k > k_last` `break`s (stops entirely, decode threads join below); inside the trim range: take an output buffer from `out_pool`; `renderer.composite_at(&pose, screen, wc_ref, &mut out)` writes the BGRA output; send the `Frame` to the encoder channel.
7. Drop the encoder channel; `join` both decode pipes (surfacing any stored decode error) and the encoder thread (surfacing any encode panic).
8. Write per-stage timing breakdown to `%TEMP%/tcursor-export-timing.txt` - `decode` is time blocked on the decode channels, `composite` is `step_camera` + `composite_at`, `encode_wait` is the channel send. With overlap, total trends toward `max(decode, composite)` rather than their sum.
9. Compute audio shift from `meta.tl.mic_ms`/`meta.tl.system_ms`, `meta.audio_offset_ms`, AND `trim_in_ms` (once trimmed, the video's own frame 0 sits `trim_in_ms` into the original capture, so both tracks shift earlier by that same amount to stay in sync); call `mux(&tmp, paths, settings.format, mic_shift, sys_shift, meta.mic_volume, meta.sys_volume)`.

### Behaviors worth knowing

- Camera shrink, CameraOnly identity override, and all per-frame compositing decisions are inside `FrameRenderer`; the export loop only drives time and I/O.
- The screen decode is a direct 1:1 pull (`ScreenPipe::next`) - output frame k is decoded frame k, with no per-frame VFR frame-selection step. This replaced an older `pipeline::advance_index`/`ScreenPipe::next_at(t)` scheme that re-timed the screen against `sync.json`; that re-timing was wrong because the CFR-60 encode has MORE frames than the recorded delivered-frame timestamps, which skewed the video against the real-time audio. The decode threads still just move bytes without changing them, so output stays byte-identical to what ffmpeg decoded.
- The trim-to-frame-index conversion is the pure `pipeline::trim_frame_bounds`, unit-tested independently (including the back-compat guard that an untrimmed clip reproduces the old bound exactly).
- Buffers are recycled through three `BufPool`s (screen decode, webcam decode, output), so the steady state allocates no new frame buffers; `BufPool::take` allocates a fresh one only under transient exhaustion rather than blocking.
- The encoder thread panic is surfaced as `Err("encoder thread panicked")` via `join().map_err(...)??`; a decode thread panic likewise via each pipe's `join`.
- Default `ExportSettings` reproduces today's export: `capture_fps`/`out_fps` collapse to the exact old formula, `settings.resolution == Source` is a no-op in `Layout::resolve`, `settings.quality_crf == 24` matches the old hardcoded CRF for `libx264`/`h264_nvenc`, and `settings.format == Mp4` takes the unchanged H.264 path. See `export::settings` and `encode::ffmpeg_args` for the guard tests.
