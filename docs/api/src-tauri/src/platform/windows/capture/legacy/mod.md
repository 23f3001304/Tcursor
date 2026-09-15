# src-tauri/src/platform/windows/capture/legacy/mod.rs

MODULE OVERVIEW: The compatibility recording path, kept for the HUD's compatibility toggle and as the automatic fallback when the Media Foundation encoder will not initialize. WGC frames are read back to CPU, pushed through `session::record::recording_session::RecordingSession` at variable rate, and written by `encode::vfr_segments::VfrSegments` through an ffmpeg rawvideo pipe. Slower and unable to survive a size change, but it records where the GPU path cannot.

`wgc_source` is the frame source; this file is the one function that wires it to a session, a sink and a thread.

Owner's ruling 4 (`docs/cross-platform-architecture.md`): this path stays Windows-only. `CaptureRequest::prefer_compatibility` is a hint other platforms ignore.

## start_ffmpeg

```rust
pub(super) fn start_ffmpeg(cfg: VideoStart, target_id: Option<&str>, video_path: &str, ox: i32, oy: i32) -> Result<(VideoSink, u32, u32, i32, i32), String>
```

`WgcFrameSource` -> `RecordingSession::run` (VFR) -> `VfrSegments`, on a `"video"` thread. The thread returns a `VideoStopped` rather than a `Result`, collecting the frame count and timestamps BEFORE `stop_and_finalize` so they survive a finalize failure.

When `run` ends while `cfg.stop` is still `false`, something other than a user Stop ended it - and `cfg.ended` fires with one of two reasons, told apart by `session.dimension_mismatch()`: `DISPLAY_CHANGED` if `RecordingSession::pump_once` latched a dimension mismatch (a mid-record window resize/maximize or display resolution/rotation change), otherwise `CAPTURE_CLOSED` because the frame source simply ran dry on its own (WGC closed the capture - recorded window closed, display unplugged). *Why the `cfg.stop` check and not an unconditional call:* a user Stop reaches the same loop exit via the halt flag and the WM_QUIT stopper, and must not be reported as an early end.

`ox` / `oy` are passed through untouched: the origin was already resolved by `start_video` before it chose a pipeline, so both paths report the same rectangle.

### Used by

- `platform/windows/capture/mod.rs` - `start_video`, both when `cfg.legacy` is set and when `GpuRecorder::start` failed.
