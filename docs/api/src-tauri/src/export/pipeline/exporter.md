# src-tauri/src/export/pipeline/exporter.rs

Top-level export orchestrator and the **composite stage** of the 3-stage decode -> composite -> encode pipeline. It loads a `FrameRenderer` from `render.rs` (which owns all per-frame compositing state), starts the screen/webcam decode threads via `ScreenPipe`/`WebcamPipe` (`pipeline.rs`), spawns the encoder thread, drives the per-frame composite loop, and muxes audio into `final.mp4`. All per-frame render logic (camera sim, compositor, FX overlays, cursor) lives in `render::FrameRenderer`; the decode threads and the encoder each run concurrently, connected by bounded channels with recycled buffer pools (`pool.rs`), so decoding frame N+1 overlaps compositing frame N overlaps encoding frame N-1. Output is byte-identical to the old sequential loop - only the overlap and buffer recycling changed.

## export

```rust
pub fn export(paths: &ProjectPaths, settings: ExportSettings, system: &dyn SystemPort, on_progress: impl Fn(u8)) -> Result<Vec<String>>
```

**Since the time remap** the loop no longer iterates recording frames between trim bounds: it takes the renderer's `TimeMap::frame_plan` (the recording frame every output frame shows; exactly `k_in ..= k_last` for a trim-only doc, skips for cuts and fast spans, repeats for slow motion) and hands the walk to `frame_loop::run` (`frame_loop.md`), which rides `FrameRenderer::walk_plan` so the camera steps once per OUTPUT frame while the decoders advance sequentially through the recording. An empty plan (everything cut) is refused with a message before the encoder is spawned. Progress, encoder timestamps and the mux length count output frames; the audio shift uses `plan[0]` where it used `k_in`, the same frame-floored trim-in, and the mux receives the kept segments (`audio_segments::audio_segs`) so the audio is cut and re-timed to match the frames.

Renders the recording at `paths` into `paths.folder/final.<ext>` (`<ext>` from `settings.format`).

### Inputs

- `paths: &ProjectPaths` - project folder root; all asset paths are derived from it (`events.json`, `actions.json`, `video.raw`, optional `webcam.raw`, mic/system audio, `sync.json`, `edit.json`, `cursor.json`). *Why a single struct:* avoids threading individual file paths through every subsystem call.
- `settings: ExportSettings` (`export::settings::ExportSettings`) - the user's chosen resolution, fps, quality (CRF), and container format. *Why one struct:* every downstream consumer (`Layout`, the encode loop, `FfmpegFrameSink`, `audio_mux::mux`) needs a different slice of it, so passing the whole struct once avoids re-deriving values. `ExportSettings::default()` reproduces today's export exactly (Source resolution, 60fps, CRF 24, MP4/H.264) - see the back-compat tests in `settings.rs`/`ffmpeg_args.rs`.
- `system: &dyn SystemPort` - the desktop facts the export build needs: `primary_refresh_hz` for `capture_fps` (step 1) and, through `FrameRenderer::new`, `os_prefers_dark`. *Why a parameter since Batch D:* both used to be free-function calls into a `#[cfg]`-gated platform module from inside the export, which is a hidden OS dependency in the one tree that is the reference for everything else. `run_export` resolves `Arc<Platform>` from app state and passes the port down; the `#[ignore]`d `export_bench` builds its own.
- `on_progress: impl Fn(u8)` - callback receiving 0..=100 each time the percentage advances. *Why a callback:* the Tauri layer (in `run.rs`) wires this to `app.emit("export-progress", p)`; keeping it external lets the export logic stay testable without a live app handle.

### Returns

`Result<Vec<String>>`. On success `final.<ext>` exists at `paths.folder` and the `Vec` carries the export's non-fatal WARNINGS - things that produced a real file but not the one the user asked for, which used to be swallowed silently. Today that is the webcam: either a decode that FAILED (`WebcamPipe::next` returning `Err`) or one that legitimately produced ZERO frames (a container this ffmpeg build cannot open, a zero-byte file from an interrupted `append_webcam`, an `-ss` past its end). Neither is worth throwing away an otherwise-correct export, since the screen is the deliverable and the camera is one panel of it. The exact wording comes from `pipeline::webcam_warning`, which distinguishes zero frames (panel ABSENT) from a part-way failure (panel present but FROZEN on the last decoded frame, via the `last_webcam` hold) - two different files that must not be described the same way. `run_export` emits each as an `export-warning` event before `export-done`.

On error the raw `anyhow::Error` propagates (including any decode-thread error surfaced by `ScreenPipe`/`WebcamPipe` - which, since `RawDecoder` now classifies a closed stdout by ffmpeg's EXIT STATUS, includes a failed screen decode carrying ffmpeg's own stderr tail).

### Implementation

1. Compute `capture_fps = system.primary_refresh_hz().min(60)` - the same display-refresh-derived value the old unconditional `fps` parameter used, now serving two roles: `FrameRenderer::new`'s capture-rate fallback (unchanged meaning, `build_timeline`'s last-resort denominator) and `settings.fps.resolve_hz(capture_fps)`'s own fallback for `Fps::Source` (so `Source` reproduces the old formula exactly without a second display query). `out_fps = settings.fps.resolve_hz(capture_fps)` is the export's actual output frame rate (`F30`/`F60` are fixed regardless of the display; `F60` is the default).
2. Call `FrameRenderer::new(paths, Layout::default(), capture_fps, settings.resolution, None, system)` to get `(renderer, meta)` (`None` = no preview downscale - a full export build). The renderer owns the event log, settings, compositor, FX renderer, camera sim, cursor state, and background. `meta` carries everything needed to drive the loop (`screen_bytes`, `webcam_w`/`webcam_h`, `video_start`, `video_end`, `tl`, `out_w`, `out_h`, `audio_offset_ms`, `trim`, `mic_volume`, `sys_volume`).
3. Open a `FfmpegFrameSink` writing to `tmp_export.<ext>` (`<ext>` from `settings.format`) via `new_medium(tmp_str, out_w, out_h, out_fps as f64, settings.format, settings.quality_crf)`. Build an `out_pool` (`BufPool`, `depth` output buffers) and spawn an encoder thread draining a `sync_channel(4)`; after pushing each `Frame` the encoder recycles its ~33 MB `bgra` buffer back into `out_pool`.
4. Start the decode threads: `ScreenPipe::spawn(..., out_fps)` (screen video) and, if `paths.webcam()` exists, `WebcamPipe::spawn(..., out_fps)` (webcam) - both decode at the resolved output rate, so either stream stays 1:1 with output frames even when `out_fps` differs from the capture rate. Spawn errors surface here. There is deliberately NO black zero-frame fallback buffer any more (it used to be an all-black nv12 frame, Y=16/U=V=128): it turned an unreadable `video.mp4` into a full-length black export that reported success.
4b. If the doc's background is a VIDEO/GIF asset that really exists (`scene::background::video_source`), open a third decode stream, `BgPipe::open(..., r.background().dim_clamped(), depth, out_fps)`. Failing to open it is logged and then ignored: the still first frame `FrameRenderer` already built stays as the whole background, which is a worse-looking export but still an export.
5. Resolve the trim gate: `full_dur_ms = meta.video_end - meta.video_start`; `meta.trim.resolve(full_dur_ms)` gives `(trim_in_ms, trim_out_ms)` (`out_ms == 0` = whole clip); `trim_frame_bounds` converts that to INCLUSIVE frame indices `(k_in, k_last)` at `out_fps`. An untrimmed clip yields `k_last` identical to the old unconditional bound, so nothing changes when there is no trim (back-compat).
6. Composite loop `0..=k_full_last` (the UNTRIMMED clip's own last index, so decode/camera-sim continuity is unaffected by trim): `bgpipe.feed(&mut r)` swaps in this frame's background - pulled on EVERY iteration, INCLUDING the pre-trim ones that are decoded but not composited, so background frame index is output frame index `k`. That is what makes a trimmed export show the same background frame the editor preview shows at the same instant (the preview's clock is the recording's own, and its frame 0 is `k = 0`); a stream that ends or fails is joined once, logged, and dropped, leaving the last frame frozen. `spipe.next()` pulls the next decoded screen frame 1:1 with output frames - the screen decode runs at `-r out_fps`, so output frame k IS decoded frame k at any export rate, with no re-timing against `sync.json` - and a `None` (only reachable on k = 0; EOF holds the last frame) is a hard error, `"screen decode produced no frames"`, not a silent black frame; `wpipe.next()` yields one webcam frame, `None` at EOF, or an `Err` - `None` keeps compositing the LAST decoded webcam frame (held in `last_webcam`, recycled through the pool once superseded) instead of letting the PiP vanish early when the webcam stream is shorter than the screen, and an `Err` is recorded once in `wc_fail` and then behaves the same (`take_err` has cleared the stored error, so later calls simply read as EOF); `renderer.step_camera(t, 1000.0 / out_fps as f32)` always advances (even outside the trim range, for continuity) - the step length is the EXACT frame period at the settings-resolved rate, not `t` minus the previous `t`, which would read 16/17/17/16 at 60fps purely because `t` is whole milliseconds (`render/mod.md`) - then `k < k_in` `continue`s (decoded/stepped but not composited) and `k > k_last` `break`s (stops entirely, decode threads join below); inside the trim range: take an output buffer from `out_pool`; `renderer.composite_at(&pose, screen, wc_ref, &mut out)` writes the BGRA output; send the `Frame` to the encoder channel.
7. Drop the encoder channel; `join` the screen pipe (surfacing any stored decode error as an `Err`), then the background pipe (log-only, same reasoning as the webcam below), then the webcam pipe - whose join error is folded into `wc_fail` as a WARNING instead, since by this point every frame is composited and encoded, so a webcam fault surfacing late must not fail a finished export - then the encoder thread (surfacing any encode panic). Build the warning from `webcam_warning(had_webcam, wc_fail, wc_frames)`.
8. `exporter_report::log_timing` writes the per-stage timing breakdown to `%TEMP%/tcursor-export-timing.txt` - `decode` is time blocked on the decode channels, `composite` is `step_camera` + `composite_at`, `encode_wait` is the channel send. With overlap, total trends toward `max(decode, composite)` rather than their sum.
9. Compute audio shift with `pipeline::audio_shift_ms` from `meta.tl.mic_ms`/`meta.tl.system_ms`, `meta.audio_offset_ms`, AND `trim_in_q = k_in * 1000 / out_fps` - the FRAME-FLOORED trim-in, not the raw `trim_in_ms`. Once trimmed, the video's own frame 0 shows the source content at `trim_in_q` (frame indices are floored in `trim_frame_bounds`), so both tracks shift earlier by that same amount; subtracting the unquantised `trim_in_ms` advanced the audio further than the picture, up to a full frame (33 ms at 30 fps) of lip-sync lead for the whole export; compute `out_dur_ms = (total_out * 1000) / out_fps` (the trimmed video's own duration, from the frame count computed in step 5); call `mux(&tmp, paths, settings.format, mic_shift, sys_shift, meta.mic_volume, meta.sys_volume, out_dur_ms)` - `out_dur_ms` caps the muxed audio to the video's actual length, so a trimmed export doesn't leave a longer source-audio tail playing past the video's frozen last frame.

### Behaviors worth knowing

- Camera shrink, CameraOnly identity override, and all per-frame compositing decisions are inside `FrameRenderer`; the export loop only drives time and I/O.
- **Honest failure over a silent black/PiP-less file.** Two failure modes used to report success: an ffmpeg decode that died on its input (indistinguishable from a clean EOF at the pipe, now separated by the exit status in `RawDecoder::read_frame`/`classify_end`) and a decode that legitimately produced zero frames. The screen is now an `Err` in both cases; the webcam is a WARNING in both (the rest of the export is still worth delivering, and `wpipe.join()` cannot re-raise the already-taken error). The cost of the trade: an ffmpeg that exits non-zero after decoding most of a file now fails the export where it previously wrote a short/frozen one.
- The screen decode is a direct 1:1 pull (`ScreenPipe::next`) - `ScreenPipe::spawn` passes `out_fps` to the `RawDecoder` (as `-r out_fps`, same as `WebcamPipe::spawn` already did), so ffmpeg itself rate-converts the decode to the export's output rate and output frame k is decoded frame k at any export fps, with no per-frame VFR frame-selection step. This replaced an older `pipeline::advance_index`/`ScreenPipe::next_at(t)` scheme that re-timed the screen against `sync.json`; that re-timing was wrong because a fixed-rate re-time has more/fewer frames than the recorded delivered-frame timestamps, which skewed the video against the real-time audio. Before the `out_fps` fix, `ScreenPipe::spawn` passed a `0.0` rate (native source rate, no `-r`), which only produced a correct 1:1 pull when `out_fps` happened to equal the capture rate - any other export fps played roughly the first `out_fps/capture_fps` fraction of the recording at the wrong speed against full-length real-time audio. The decode threads still just move bytes without changing them, so output stays byte-identical to what ffmpeg decoded.
- The trim-to-frame-index conversion is the pure `pipeline::trim_frame_bounds`, unit-tested independently (including the back-compat guard that an untrimmed clip reproduces the old bound exactly).
- Buffers are recycled through three `BufPool`s (screen decode, webcam decode, output), so the steady state allocates no new frame buffers; `BufPool::take` allocates a fresh one only under transient exhaustion rather than blocking.
- The encoder thread panic is surfaced as `Err("encoder thread panicked")` via `join().map_err(...)??`; a decode thread panic likewise via each pipe's `join`.
- The manual benches (`preview_frame_bench` and `export_bench`, both `#[ignore]`d and needing `TCURSOR_REC`) live in this file's own `#[cfg(test)] mod bench`; the progress/timing reporting lives in `exporter_report.rs` (`tick_progress`, `log_timing`). Run a bench with `cargo test export_bench -- --ignored --nocapture`.
- Default `ExportSettings` reproduces today's export: `capture_fps`/`out_fps` collapse to the exact old formula, `settings.resolution == Source` is a no-op in `Layout::resolve`, `settings.quality_crf == 24` matches the old hardcoded CRF for `libx264`/`h264_nvenc`, and `settings.format == Mp4` takes the unchanged H.264 path. See `export::settings` and `encode::ffmpeg_args` for the guard tests.

## preview_frame_bench

```rust
#[test]
#[ignore]
fn preview_frame_bench()
```

One composited frame of a real recording to `%TEMP%/tcursor-preview-frame.png`, through the editor's own `render_preview` (the GPU compositor; no video encoder is involved). `TCURSOR_REC` names the folder, `TCURSOR_MS` the instant (default 1500):

```
TCURSOR_REC=C:\path\to\a\recording TCURSOR_MS=2500 cargo test preview_frame_bench -- --ignored --nocapture
```

Made for 2026-09-14's odd-sized capture, whose export slid and sheared: a look at one frame says whether a decode is sound without occupying the machine's hardware encoder for a full export (which, run while the owner was recording, had already cost one take its video).

## export_bench

```rust
#[test]
#[ignore]
fn export_bench()
```

Runs a FULL export of a real recording folder and prints how long it took. `#[ignore]`d because it needs a recording that only exists on a developer's machine and takes minutes:

```
TCURSOR_REC=C:\path\to\a\recording cargo test export_bench -- --ignored --nocapture
```

Reads the folder from `TCURSOR_REC` (panics with that instruction if unset), builds a `ProjectPaths` from it, and calls `export` with `ExportSettings::default()` and a no-op progress callback. The per-stage breakdown it is usually paired with is the one `export` writes to `%TEMP%/tcursor-export-timing.txt`.

### Used by

Nothing in the build - both benches are opt-in measuring tools, not regression tests.
