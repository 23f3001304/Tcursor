use super::super::target::capture_window_size;
use super::frames::{Cap, CapFlags, EncoderSpec, FrameTimes};
use crate::domain::time::Clock;
use crate::ports::capture::TargetId;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::video_sink::VideoStopped;
use crate::session::record::Notify;
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
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings, TryIntoCaptureItemWithType,
};

pub(crate) fn target_bitrate(w: u32, h: u32) -> u32 {
    (w as u64 * h as u64 * 12).clamp(8_000_000, 80_000_000) as u32
}

fn video_settings(w: u32, h: u32, fps: u32) -> VideoSettingsBuilder {
    VideoSettingsBuilder::new(w, h)
        .sub_type(VideoSettingsSubType::H264)
        .bitrate(target_bitrate(w, h))
        .frame_rate(fps)
}

pub(super) fn encoder(w: u32, h: u32, fps: u32, video_path: &str) -> anyhow::Result<VideoEncoder> {
    Ok(VideoEncoder::new(
        video_settings(w, h, fps),
        AudioSettingsBuilder::default().disabled(true),
        ContainerSettingsBuilder::default(),
        video_path,
    )?)
}

pub struct GpuRecorder {
    pub(super) control: CaptureControl<Cap, anyhow::Error>,
    pub(super) frame_ts: FrameTimes,
}

pub struct GpuStart {
    pub clock: Arc<dyn Clock>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}

fn launch<T: TryIntoCaptureItemWithType + Send + 'static>(
    item: T,
    cfg: &GpuStart,
    flags: CapFlags,
) -> anyhow::Result<CaptureControl<Cap, anyhow::Error>> {
    let cursor = if cfg.with_cursor {
        CursorCaptureSettings::WithCursor
    } else {
        CursorCaptureSettings::WithoutCursor
    };
    let interval = MinimumUpdateIntervalSettings::Custom(std::time::Duration::from_micros(
        1_000_000 / cfg.fps.max(1) as u64,
    ));
    Ok(Cap::start_free_threaded(Settings::new(
        item,
        cursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        interval,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        flags,
    ))?)
}

pub(super) fn start_capture(
    cfg: &GpuStart,
    target_id: Option<&str>,
    flags: CapFlags,
) -> anyhow::Result<(CaptureControl<Cap, anyhow::Error>, u32, u32)> {
    use windows_capture::window::Window;

    match TargetId::from_arg(target_id) {
        TargetId::Window(handle) => {
            let hwnd = windows::Win32::Foundation::HWND(handle as usize as *mut _);
            let (w, h) = capture_window_size(hwnd);
            let win = Window::from_raw_hwnd(hwnd.0 as *mut _);
            return Ok((launch(win, cfg, flags)?, w, h));
        }
        TargetId::Display(index) => {
            if let Ok(monitor) = Monitor::from_index(index) {
                let (w, h) = (
                    monitor.width().unwrap_or(1920),
                    monitor.height().unwrap_or(1080),
                );
                return Ok((launch(monitor, cfg, flags)?, w, h));
            }
        }
        TargetId::Primary => {}
    }

    let monitor = Monitor::primary()?;
    let (w, h) = (monitor.width()?, monitor.height()?);
    Ok((launch(monitor, cfg, flags)?, w, h))
}

impl GpuRecorder {
    pub fn start(
        cfg: GpuStart,
        target_id: Option<&str>,
        video_path: &str,
    ) -> anyhow::Result<(Self, u32, u32)> {
        let frame_ts: FrameTimes = Arc::new(std::sync::Mutex::new(Vec::new()));
        let flags = CapFlags {
            enc: EncoderSpec {
                fps: cfg.fps,
                path: video_path.to_string(),
            },
            clock: cfg.clock.clone(),
            frame_ts: frame_ts.clone(),
            paused: cfg.paused.clone(),
            totals: cfg.totals.clone(),
            ended: cfg.ended.clone(),
            seed: None,
            on_size: None,
        };
        let (control, w, h) = start_capture(&cfg, target_id, flags)?;
        Ok((Self { control, frame_ts }, w, h))
    }

    pub fn frame_times(&self) -> FrameTimes {
        self.frame_ts.clone()
    }

    pub fn stop(self) -> VideoStopped {
        let Self { control, frame_ts } = self;
        let cap = control.callback();
        let mut error = control
            .stop()
            .err()
            .map(|e| format!("gpu capture stop: {e:?}"));
        let enc = cap.lock().encoder.take();
        if let Some(e) = enc {
            if let Err(e) = e.finish() {
                error.get_or_insert(format!("gpu encode finalize: {e:?}"));
            }
        }
        let frame_ts = frame_ts.lock().unwrap_or_else(|e| e.into_inner()).clone();
        VideoStopped {
            frames: frame_ts.len() as u64,
            frame_ts,
            error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitrate_scales_with_pixels_and_clamps() {
        assert_eq!(target_bitrate(1920, 1080), 24_883_200);
        assert_eq!(target_bitrate(320, 240), 8_000_000);
        assert_eq!(target_bitrate(3840, 2160), 80_000_000);
    }
}
