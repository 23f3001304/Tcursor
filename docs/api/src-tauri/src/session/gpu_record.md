# src-tauri/src/session/gpu_record.rs

GPU-native recording: feeds each WGC frame's D3D11 surface straight into the `windows-capture` Media Foundation `VideoEncoder` (no GPU->CPU readback), curing game-capture lag. The recorded `video.mp4` is the raw full-res intermediate the export re-composites; per-frame capture timestamps are collected here for `sync.json`. This is the default recording path (picked by `video_sink::start_video`); the ffmpeg pipe is the fallback.

## target_bitrate

```rust
pub(crate) fn target_bitrate(w: u32, h: u32) -> u32
```

Target H.264 bitrate (bits/s) for the raw recording intermediate at `w`x`h`: `w*h*12` clamped to `[8 Mbps, 80 Mbps]`. Deliberately generous - the export re-encodes this, so compression loss must not be baked into the master - but capped so 4K stays reasonable (~25 Mbps at 1080p; the 80 Mbps cap binds at 4K).

### Behaviors
- `bitrate_scales_with_pixels_and_clamps`: 1920x1080 -> 24_883_200; 320x240 -> 8_000_000 (min clamp); 3840x2160 -> 80_000_000 (max clamp).

## GpuRecorder

```rust
pub struct GpuRecorder { /* CaptureControl<Cap>, frame_ts: Arc<Mutex<Vec<u64>>> */ }
```

A live GPU-native recording. The WGC capture + MF encode run together on the crate's own thread: the private `Cap` handler owns the `VideoEncoder` and encodes each frame's surface in `on_frame_arrived` (recording `clock.now_ms()` per frame, skipping while paused). The MP4 is finalized by `GpuRecorder::stop` (the crate does not run `on_closed` on a WM_QUIT stop). This struct holds the control handle and the shared per-frame timestamps.

## GpuRecorder::start

```rust
pub fn start(clock: Arc<dyn Clock>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, video_path: &str) -> anyhow::Result<(Self, u32, u32)>
```

Starts GPU-native capture+encode of the primary monitor to `video_path` (H.264 MP4; audio disabled - audio is a separate pipeline muxed at export). Returns the recorder plus the captured `(w, h)`. Errors (so the caller can fall back to ffmpeg) when the monitor, the MF H.264 encoder, or the capture can't initialize.

## GpuRecorder::stop

```rust
pub fn stop(self) -> Result<Vec<u64>, String>
```

Stops capture and finalizes the MP4: `CaptureControl::stop` posts WM_QUIT and joins the capture thread, then the encoder is finalized **explicitly** (`finish()`) with its error surfaced - the crate does NOT call `on_closed` on a WM_QUIT stop, and the encoder's `Drop` would otherwise swallow a tail encode/mux failure on the unrecoverable master. Returns the per-frame capture timestamps (ms, in order) for `sync.json`.
