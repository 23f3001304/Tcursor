//! GPU-native recording: feed each WGC frame's D3D11 surface straight into the windows-capture
//! Media Foundation `VideoEncoder` (no GPU->CPU readback), curing game-recording lag. The
//! recorded `video.mp4` is the raw full-res intermediate the export re-composites; per-frame
//! capture timestamps still go to `sync.json` (collected here, written by `recorder_stop.rs`).
//! The frame callback itself lives in `gpu_frames.rs`.
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use windows_capture::capture::{CaptureControl, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder,
    VideoSettingsSubType,
};
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use crate::domain::time::Clock;
use super::gpu_frames::{Cap, CapFlags, FrameTimes};
use super::pause_totals::PauseTotals;
use super::video_sink::VideoStopped;
use super::Notify;

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

/// The MP4 encoder for a `w`x`h`@`fps` capture writing `video_path`. Audio is disabled - the
/// mic and system-audio WAVs are captured separately and muxed at export.
pub(super) fn encoder(w: u32, h: u32, fps: u32, video_path: &str) -> anyhow::Result<VideoEncoder> {
    Ok(VideoEncoder::new(
        video_settings(w, h, fps),
        AudioSettingsBuilder::default().disabled(true),
        ContainerSettingsBuilder::default(),
        video_path,
    )?)
}

/// A live GPU-native recording. The capture+encode runs on the crate's own thread; `stop` ends
/// it, finalizes the MP4, and returns the per-frame capture timestamps (ms) for `sync.json`.
pub struct GpuRecorder {
    control: CaptureControl<Cap, anyhow::Error>,
    frame_ts: FrameTimes,
}

/// Everything `start` needs that isn't the capture target itself, so the three target branches
/// below stay one line each.
pub struct GpuStart {
    pub clock: Arc<dyn Clock>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}

impl GpuRecorder {
    /// Start GPU-native capture+encode of the specified monitor or application window to
    /// `video_path` (H.264 MP4). Returns the recorder plus the captured `(w, h)`.
    pub fn start(cfg: GpuStart, target_id: Option<&str>, video_path: &str) -> anyhow::Result<(Self, u32, u32)> {
        use windows_capture::window::Window;

        let cursor_setting = if cfg.with_cursor { CursorCaptureSettings::WithCursor } else { CursorCaptureSettings::WithoutCursor };
        let interval_setting = MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(1_000_000 / cfg.fps.max(1) as u64));
        let frame_ts: FrameTimes = Arc::new(std::sync::Mutex::new(Vec::new()));
        // The encoder is NOT built here: `GetWindowRect`/monitor dims are only an estimate of
        // what WGC will actually deliver, and a wrong guess corrupts every frame. `Cap` builds it
        // from the first real frame instead; this just carries the settings it needs.
        let flags = || CapFlags {
            enc: super::gpu_frames::EncoderSpec { fps: cfg.fps, path: video_path.to_string() },
            clock: cfg.clock.clone(), frame_ts: frame_ts.clone(),
            paused: cfg.paused.clone(), totals: cfg.totals.clone(), ended: cfg.ended.clone(),
        };

        if let Some(tid) = target_id {
            if let Some(hex) = tid.strip_prefix("window:0x") {
                if let Ok(hwnd_val) = usize::from_str_radix(hex, 16) {
                    let hwnd = windows::Win32::Foundation::HWND(hwnd_val as *mut _);
                    let win = Window::from_raw_hwnd(hwnd.0 as *mut _);
                    let mut r = windows::Win32::Foundation::RECT::default();
                    let (w, h) = if unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut r) }.is_ok() {
                        ((r.right - r.left).max(100) as u32, (r.bottom - r.top).max(100) as u32)
                    } else {
                        (1920, 1080)
                    };
                    let settings = Settings::new(
                        win,
                        cursor_setting,
                        DrawBorderSettings::WithoutBorder,
                        SecondaryWindowSettings::Default,
                        interval_setting,
                        DirtyRegionSettings::Default,
                        ColorFormat::Bgra8,
                        flags(),
                    );
                    return Ok((Self { control: Cap::start_free_threaded(settings)?, frame_ts }, w, h));
                }
            } else if let Some(idx_str) = tid.strip_prefix("display:") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Ok(mon) = Monitor::from_index(idx) {
                        let w = mon.width().unwrap_or(1920);
                        let h = mon.height().unwrap_or(1080);
                        let settings = Settings::new(
                            mon,
                            cursor_setting,
                            DrawBorderSettings::WithoutBorder,
                            SecondaryWindowSettings::Default,
                            interval_setting,
                            DirtyRegionSettings::Default,
                            ColorFormat::Bgra8,
                            flags(),
                        );
                        return Ok((Self { control: Cap::start_free_threaded(settings)?, frame_ts }, w, h));
                    }
                }
            }
        }

        let monitor = Monitor::primary()?;
        let (w, h) = (monitor.width()?, monitor.height()?);
        let settings = Settings::new(
            monitor,
            cursor_setting,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            interval_setting,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            flags(),
        );
        Ok((Self { control: Cap::start_free_threaded(settings)?, frame_ts }, w, h))
    }

    /// Stop capture and finalize the MP4. `CaptureControl::stop` posts WM_QUIT and joins the
    /// capture thread; the crate runs `on_closed` only on an OS-initiated close, NOT on a
    /// WM_QUIT stop, so the encoder is finalized explicitly here and its finalize error
    /// surfaced - relying on the encoder's `Drop` would silently swallow a tail encode/mux
    /// failure on the (unrecoverable) master. The frame timestamps come back either way: they
    /// live in their own `Arc`, so a finalize failure still leaves `stop_recording` able to
    /// write a truthful `sync.json` for whatever the file did capture.
    pub fn stop(self) -> VideoStopped {
        let Self { control, frame_ts } = self;
        let cap = control.callback(); // Arc<Mutex<Cap>> - grab before stop() consumes control
        let mut error = control.stop().err().map(|e| format!("gpu capture stop: {e:?}"));
        let enc = cap.lock().encoder.take(); // crate's callback() is a parking_lot Mutex (no poison)
        if let Some(e) = enc {
            if let Err(e) = e.finish() { error.get_or_insert(format!("gpu encode finalize: {e:?}")); }
        }
        let frame_ts = frame_ts.lock().unwrap_or_else(|e| e.into_inner()).clone();
        VideoStopped { frames: frame_ts.len() as u64, frame_ts, error }
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
