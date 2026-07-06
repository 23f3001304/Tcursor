//! GPU-native recording: feed each WGC frame's D3D11 surface straight into the windows-capture
//! Media Foundation `VideoEncoder` (no GPU->CPU readback), curing game-recording lag. The
//! recorded `video.mp4` is the raw full-res intermediate the export re-composites; per-frame
//! capture timestamps still go to `sync.json` (collected here, written by `recorder.rs`).
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
    VideoSettingsSubType,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use crate::domain::time::Clock;

/// Target H.264 bitrate (bits/s) for the raw recording intermediate at `w`x`h`: scaled by pixel
/// count and clamped to a sane range. Deliberately generous (the export re-encodes this, so we
/// don't want to bake compression loss into the master), but capped so 4K stays reasonable.
pub(crate) fn target_bitrate(w: u32, h: u32) -> u32 {
    (w as u64 * h as u64 * 12).clamp(8_000_000, 80_000_000) as u32
}

/// H.264 video settings for the GPU encoder at `w`x`h`@`fps`.
fn video_settings(w: u32, h: u32, fps: u32) -> VideoSettingsBuilder {
    VideoSettingsBuilder::new(w, h)
        .sub_type(VideoSettingsSubType::H264)
        .bitrate(target_bitrate(w, h))
        .frame_rate(fps)
}

type FrameTimes = Arc<Mutex<Vec<u64>>>;

/// The WGC capture handler that hardware-encodes each frame on the GPU (no readback) and records
/// its capture time. The encoder is finalized by `GpuRecorder::stop` (or in `on_closed` if the OS
/// closes the capture).
struct Cap {
    encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
}

impl GraphicsCaptureApiHandler for Cap {
    type Flags = (VideoEncoder, Arc<dyn Clock>, FrameTimes, Arc<AtomicBool>);
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let (encoder, clock, frame_ts, paused) = ctx.flags;
        Ok(Self { encoder: Some(encoder), clock, frame_ts, paused })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _ctl: InternalCaptureControl) -> Result<(), Self::Error> {
        // Skip frames while paused (matches the ffmpeg path); record the capture time, then send
        // the frame's D3D11 surface straight to the GPU encoder (send_frame blocks until the GPU
        // drains it - natural back-pressure). The ts is pushed BEFORE send_frame: a send error
        // aborts capture and stop() surfaces it, so the orphan ts never reaches sync.json.
        if !self.paused.load(Ordering::SeqCst) {
            self.frame_ts.lock().unwrap_or_else(|e| e.into_inner()).push(self.clock.now_ms());
            if let Some(e) = self.encoder.as_mut() { e.send_frame(frame)?; }
        }
        Ok(())
    }

    // Runs only when the OS closes the capture (not on a WM_QUIT stop, where GpuRecorder::stop
    // finalizes); finalize here too so an OS-close still writes a valid MP4.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        if let Some(e) = self.encoder.take() { e.finish()?; }
        Ok(())
    }
}

/// A live GPU-native recording. The capture+encode runs on the crate's own thread; `stop` ends
/// it, finalizes the MP4, and returns the per-frame capture timestamps (ms) for `sync.json`.
pub struct GpuRecorder {
    control: CaptureControl<Cap, anyhow::Error>,
    frame_ts: FrameTimes,
}

impl GpuRecorder {
    /// Start GPU-native capture+encode of the primary monitor to `video_path` (H.264 MP4).
    /// Returns the recorder plus the captured `(w, h)`. Audio is disabled (recorded separately).
    pub fn start(clock: Arc<dyn Clock>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, video_path: &str) -> anyhow::Result<(Self, u32, u32)> {
        let monitor = Monitor::primary()?;
        let (w, h) = (monitor.width()?, monitor.height()?);
        let encoder = VideoEncoder::new(
            video_settings(w, h, fps),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            video_path,
        )?;
        let frame_ts: FrameTimes = Arc::new(Mutex::new(Vec::new()));
        let settings = Settings::new(
            monitor,
            if with_cursor { CursorCaptureSettings::WithCursor } else { CursorCaptureSettings::WithoutCursor },
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(1_000_000 / fps.max(1) as u64)),
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            (encoder, clock, frame_ts.clone(), paused),
        );
        Ok((Self { control: Cap::start_free_threaded(settings)?, frame_ts }, w, h))
    }

    /// Stop capture and finalize the MP4, returning the per-frame capture timestamps for
    /// `sync.json`. `CaptureControl::stop` posts WM_QUIT and joins the capture thread; the crate
    /// runs `on_closed` only on an OS-initiated close, NOT on a WM_QUIT stop, so the encoder is
    /// finalized explicitly here and its finalize error surfaced - relying on the encoder's `Drop`
    /// would silently swallow a tail encode/mux failure on the (unrecoverable) master.
    pub fn stop(self) -> Result<Vec<u64>, String> {
        let cap = self.control.callback(); // Arc<Mutex<Cap>> - grab before stop() consumes control
        self.control.stop().map_err(|e| format!("gpu capture stop: {e:?}"))?;
        let enc = cap.lock().encoder.take(); // crate's callback() is a parking_lot Mutex (no poison)
        if let Some(e) = enc { e.finish().map_err(|e| format!("gpu encode finalize: {e:?}"))?; }
        Ok(self.frame_ts.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitrate_scales_with_pixels_and_clamps() {
        assert_eq!(target_bitrate(1920, 1080), 24_883_200); // ~25 Mbps for 1080p (2.07M px * 12)
        assert_eq!(target_bitrate(320, 240), 8_000_000); // tiny -> min clamp
        assert_eq!(target_bitrate(3840, 2160), 80_000_000); // 4K (99.5M raw) -> max clamp
    }
}
