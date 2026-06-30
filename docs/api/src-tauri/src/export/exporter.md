# src-tauri/src/export/exporter.rs

Top-level export orchestrator: loads a `FrameRenderer` from `render.rs` (which owns all per-frame compositing state), spawns raw decoders and an encoder thread, drives the per-frame loop, and muxes audio into `final.mp4`. All per-frame render logic (camera sim, compositor, FX overlays, cursor) has been extracted into `render::FrameRenderer`; this file owns only the decoders, the encoder channel, the timing diagnostics, and the audio mux.

## export

```rust
pub fn export(paths: &ProjectPaths, fps: u32, on_progress: impl Fn(u8)) -> Result<()>
```

Renders the recording at `paths` into `paths.folder/final.mp4`.

### Inputs

- `paths: &ProjectPaths` - project folder root; all asset paths are derived from it (`events.json`, `actions.json`, `video.raw`, optional `webcam.raw`, mic/system audio, `sync.json`, `edit.json`, `cursor.json`). *Why a single struct:* avoids threading individual file paths through every subsystem call.
- `fps: u32` - the capture frame rate from the recording session. *Why:* forwarded to `FrameRenderer::new`, which passes it to `build_timeline` as a last-resort denominator when `sync.json` is absent and no audio duration is available.
- `on_progress: impl Fn(u8)` - callback receiving 0..=100 each time the percentage advances. *Why a callback:* the Tauri layer (in `run.rs`) wires this to `app.emit("export-progress", p)`; keeping it external lets the export logic stay testable without a live app handle.

### Returns

`Result<()>`. On success `final.mp4` exists at `paths.folder`. On error the raw `anyhow::Error` propagates to the caller.

### Implementation

1. Call `FrameRenderer::new(paths, Layout::default(), fps)` to get `(renderer, meta)`. The renderer owns the event log, settings, compositor, FX renderer, camera sim, cursor state, and background. `meta` carries everything needed to spawn decoders (`screen_bytes`, `webcam_size`, `video_start`, `tl`, `out_w`, `out_h`, `audio_offset_ms`).
2. Spawn `RawDecoder` threads for the screen video and (if `paths.webcam()` exists) the webcam, using sizes from `meta`.
3. Open a `FfmpegFrameSink` writing to `tmp_export.mp4`. Spawn an encoder thread draining a `sync_channel(4)` so compositing the next frame overlaps with encoding the previous one.
4. Frame loop `0..=total_out` (derived from `meta.video_end - meta.video_start`): advance the screen decoder to the captured frame active at output time `t`; read one webcam frame via `read_webcam`; call `renderer.step_camera(t)` to get the `FramePose`; call `renderer.composite_at(&pose, &screen_buf, webcam)` to get the BGRA output; send `Frame` to the encoder channel.
5. Drop the channel to signal the encoder; join the encoder thread (surfaces any encode panic).
6. Write per-stage timing breakdown to `%TEMP%/tcursor-export-timing.txt` (decode / composite / encode-wait in ms).
7. Compute audio shift from `meta.tl.mic_ms`/`meta.tl.system_ms` and `meta.audio_offset_ms`; call `mux(&tmp, paths, mic_shift, sys_shift)`.

### Behaviors worth knowing

- Camera shrink, CameraOnly identity override, and all per-frame compositing decisions are now inside `FrameRenderer`; the export loop only drives time and I/O.
- The encoder thread panic is surfaced as `Err("encoder thread panicked")` via `join().map_err(...)??`.

## read_webcam

```rust
fn read_webcam<'a>(
    dec: &mut Option<RawDecoder>,
    buf: &'a mut [u8],
    size: u32,
) -> Result<Option<(&'a [u8], u32, u32)>>
```

Reads one webcam frame into `buf`, returning a slice reference. On EOF or when no decoder is present, sets `*dec = None` and returns `Ok(None)` so all subsequent calls are no-ops.

### Inputs

- `dec: &mut Option<RawDecoder>` - the webcam decoder slot. *Why mutable Option:* on EOF the decoder is dropped in-place so the compositors cleanly skip the camera panel for all remaining frames without a separate EOF flag.
- `buf: &'a mut [u8]` - pre-allocated buffer (`size * size * 4` bytes). *Why reused:* avoids a per-frame heap allocation for the largest panel in the output.
- `size: u32` - square side length. *Why square:* webcam decode is capped to a square to bound memory; the compositor expects square input.

### Returns

`Result<Option<(&'a [u8], u32, u32)>>` - `Some((slice, size, size))` while frames remain; `None` on EOF.
