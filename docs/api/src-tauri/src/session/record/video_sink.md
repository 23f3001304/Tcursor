# src-tauri/src/session/record/video_sink.rs

Picks the recording video pipeline: GPU-native Media Foundation by default (cures game-capture lag - no per-frame readback), falling back to the legacy ffmpeg rawvideo pipe when the user forces it (the compatibility toggle) or the GPU encoder won't initialize. Both write `video.mp4` and return the per-frame capture timestamps (ms) the export needs for `sync.json`.

## VideoSink

```rust
pub enum VideoSink { Gpu(GpuRecorder), Ffmpeg { thread, halt, stopper } }
```

A live recording video pipeline. `Gpu` is the GPU-native path (`GpuRecorder`); `Ffmpeg` is the legacy WGC-readback -> `RecordingSession` -> `FfmpegFrameSink` path running on its own thread, kept as the manual + automatic fallback.

## start_video

```rust
pub fn start_video(legacy: bool, clock: Arc<dyn Clock>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, video_path: &str) -> Result<(VideoSink, u32, u32), String>
```

Starts the video pipeline writing `video_path`. GPU-native unless `legacy` is set (the compatibility toggle); on a `GpuRecorder::start` error it logs and falls back to ffmpeg, so recording never simply fails. Returns the sink plus the captured `(w, h)`. The ffmpeg branch builds `WgcFrameSource` + `FfmpegFrameSink::new_vfr` and runs `RecordingSession::run` (VFR) on a `"video"` thread.

## VideoSink::stop_and_collect

```rust
pub fn stop_and_collect(self) -> Result<(u64, Vec<u64>), String>
```

Stops the pipeline and finalizes `video.mp4`, returning `(frame count, per-frame capture timestamps ms)`. GPU: delegates to `GpuRecorder::stop`. Ffmpeg: sets the halt flag, runs the stopper (WM_QUIT unblocks the WGC thread, closing the frame channel so `run()` ends), then joins the thread.
