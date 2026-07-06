# src-tauri/src/export/pipeline/exporter.rs

Top-level export orchestrator and the **composite stage** of the 3-stage decode -> composite -> encode pipeline. It loads a `FrameRenderer` from `render.rs` (which owns all per-frame compositing state), starts the screen/webcam decode threads via `ScreenPipe`/`WebcamPipe` (`pipeline.rs`), spawns the encoder thread, drives the per-frame composite loop, and muxes audio into `final.mp4`. All per-frame render logic (camera sim, compositor, FX overlays, cursor) lives in `render::FrameRenderer`; the decode threads and the encoder each run concurrently, connected by bounded channels with recycled buffer pools (`pool.rs`), so decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1. Output is byte-identical to the old sequential loop - only the overlap and buffer recycling changed.

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

`Result<()>`. On success `final.mp4` exists at `paths.folder`. On error the raw `anyhow::Error` propagates to the caller (including any decode-thread error surfaced by `ScreenPipe`/`WebcamPipe`).

### Implementation

1. Call `FrameRenderer::new(paths, Layout::default(), fps)` to get `(renderer, meta)`. The renderer owns the event log, settings, compositor, FX renderer, camera sim, cursor state, and background. `meta` carries everything needed to drive the loop (`screen_bytes`, `webcam_size`, `video_start`, `tl`, `out_w`, `out_h`, `audio_offset_ms`).
2. Open a `FfmpegFrameSink` writing to `tmp_export.mp4`. Build an `out_pool` (`BufPool`, `depth` output buffers) and spawn an encoder thread draining a `sync_channel(4)`; after pushing each `Frame` the encoder recycles its ~33 MB `bgra` buffer back into `out_pool`.
3. Start the decode threads: `ScreenPipe::spawn` (screen video) and, if `paths.webcam()` exists, `WebcamPipe::spawn` (webcam). Spawn errors surface here. Allocate a zeroed `empty` screen buffer as the zero-frame fallback.
4. Composite loop `0..=total_out` (derived from `meta.video_end - meta.video_start`): `spipe.next_at(t)` yields the captured screen frame active at output time `t` (or `empty` for a zero-frame video); `wpipe.next()` yields one webcam frame (or `None` at EOF); `renderer.step_camera(t)` gives the `FramePose`; take an output buffer from `out_pool`; `renderer.composite_at(&pose, screen, wc_ref, &mut out)` writes the BGRA output; send the `Frame` to the encoder channel; then `recycle` the webcam buffer.
5. Drop the encoder channel; `join` both decode pipes (surfacing any stored decode error) and the encoder thread (surfacing any encode panic).
6. Write per-stage timing breakdown to `%TEMP%/tcursor-export-timing.txt` - `decode` is time blocked on the decode channels, `composite` is `step_camera` + `composite_at`, `encode_wait` is the channel send. With overlap, total trends toward `max(decode, composite)` rather than their sum.
7. Compute audio shift from `meta.tl.mic_ms`/`meta.tl.system_ms` and `meta.audio_offset_ms`; call `mux(&tmp, paths, mic_shift, sys_shift)`.

### Behaviors worth knowing

- Camera shrink, CameraOnly identity override, and all per-frame compositing decisions are inside `FrameRenderer`; the export loop only drives time and I/O.
- The per-frame VFR screen-frame selection is the pure `pipeline::advance_index`, unit-tested independently; the decode threads move bytes without changing them, so output stays byte-identical.
- Buffers are recycled through three `BufPool`s (screen decode, webcam decode, output), so the steady state allocates no new frame buffers; `BufPool::take` allocates a fresh one only under transient exhaustion rather than blocking.
- The encoder thread panic is surfaced as `Err("encoder thread panicked")` via `join().map_err(...)??`; a decode thread panic likewise via each pipe's `join`.
