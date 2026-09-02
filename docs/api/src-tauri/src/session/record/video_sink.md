# src-tauri/src/session/record/video_sink.rs

Picks the recording video pipeline: GPU-native Media Foundation by default (cures game-capture lag - no per-frame readback), falling back to the legacy ffmpeg rawvideo pipe when the user forces it (the compatibility toggle) or the GPU encoder won't initialize. Both write `video.mp4` and return the per-frame capture timestamps (ms) the export needs for `sync.json`.

## VideoSink

```rust
pub enum VideoSink { Gpu(GpuRecorder), Ffmpeg { thread, halt, stopper } }
```

A live recording video pipeline. `Gpu` is the GPU-native path (`GpuRecorder`); `Ffmpeg` is the legacy WGC-readback -> `RecordingSession` -> `VfrSegments` path running on its own thread, kept as the manual + automatic fallback.

## VideoStopped

```rust
pub struct VideoStopped {
    pub frames: u64,
    pub frame_ts: Vec<u64>,
    pub error: Option<String>,
}
```

What a stopped video pipeline leaves behind.

- `frames: u64` - successfully encoded frame count.
- `frame_ts: Vec<u64>` - per-frame capture times (ms, pause-compressed) for `sync.json`.
- `error: Option<String>` - a stop/finalize failure. *Why reported alongside the timestamps instead of replacing them:* a finalize failure used to propagate out of the stop path before `sync.json`, `project.tcursor` and the recents entry were written, so an encoder/mux tail failure (disk full on a long take) left a folder that `open_project`'s `*.tcursor` filter cannot even select and that `build_timeline` would have had to synthesise a timeline for (finding M1). The frames that were recorded exist on disk either way, so the caller writes those files from `frame_ts` first and surfaces `error` after.

## VideoStart

```rust
pub struct VideoStart {
    pub legacy: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}
```

Everything both capture paths need beside the target and the output path.

- `legacy: bool` - force the ffmpeg pipeline (the HUD's compatibility toggle).
- `totals: Arc<PauseTotals>` - the exact-span pause ledger both the video timestamps and every input stream subtract, so they cannot disagree about a pause.
- `ended: Notify` - called if the OS ends the capture on its own, OR the capture's own dimensions change mid-record, so the app can run its normal Stop instead of leaving the HUD counting against a video that already stopped.

## start_video

```rust
pub fn start_video(cfg: VideoStart, target_id: Option<&str>, video_path: &str) -> Result<(VideoSink, u32, u32, i32, i32), String>
```

Starts the video pipeline writing `video_path`. GPU-native unless `cfg.legacy` is set; on a `GpuRecorder::start` error it logs and falls back to ffmpeg, so recording never simply fails. Returns the sink plus the captured `(w, h, origin_x, origin_y)`, the origin coming from `get_target_bounds`.

## start_ffmpeg

```rust
fn start_ffmpeg(cfg: VideoStart, target_id: Option<&str>, video_path: &str, ox: i32, oy: i32) -> Result<(VideoSink, u32, u32, i32, i32), String>
```

The legacy path: `WgcFrameSource` -> `RecordingSession::run` (VFR) -> `VfrSegments`, on a `"video"` thread. The thread returns a `VideoStopped` rather than a `Result`, collecting the frame count and timestamps BEFORE `stop_and_finalize` so they survive a finalize failure.

When `run` ends while `cfg.stop` is still `false`, something other than a user Stop ended it - and `cfg.ended` fires with one of two reasons, told apart by `session.dimension_mismatch()`: `DISPLAY_CHANGED` if `RecordingSession::pump_once` latched a dimension mismatch (Task 5, finding H1 - a mid-record window resize/maximize or display resolution/rotation change), otherwise `CAPTURE_CLOSED` because the frame source simply ran dry on its own (WGC closed the capture - recorded window closed, display unplugged). *Why the `cfg.stop` check and not an unconditional call:* a user Stop reaches the same loop exit via the halt flag and the WM_QUIT stopper, and must not be reported as an early end.

## VideoSink::stop_and_collect

```rust
pub fn stop_and_collect(self) -> VideoStopped
```

Stops the pipeline and finalizes `video.mp4`. GPU: delegates to `GpuRecorder::stop`. Ffmpeg: sets the halt flag, runs the stopper (WM_QUIT unblocks the WGC thread, closing the frame channel so `run()` ends), then joins the thread - a panicked thread becomes a `VideoStopped` with an `error` and no frames, rather than being propagated as a `Result`, so the caller's salvage path is identical in every failure mode.
