//! Picks the recording video pipeline: GPU-native Media Foundation by default (cures game-capture
//! lag - no per-frame readback), falling back to the legacy ffmpeg rawvideo pipe when the user
//! forces it (the compatibility toggle) or the GPU encoder won't initialize. Both write
//! `video.mp4` and return the per-frame capture timestamps (ms) the export needs for `sync.json`.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use crate::capture::frame_source::FrameSource;
use crate::capture::windows_capture::WgcFrameSource;
use crate::domain::time::Clock;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::session::record::gpu_record::GpuRecorder;
use crate::session::record::recording_session::RecordingSession;

/// A live recording video pipeline.
pub enum VideoSink {
    /// GPU-native: the windows-capture Media Foundation encoder, hardware-encoding on the GPU.
    Gpu(GpuRecorder),
    /// Legacy fallback: WGC readback -> `RecordingSession` -> ffmpeg rawvideo pipe, on a thread.
    Ffmpeg {
        thread: JoinHandle<std::io::Result<(u64, Vec<u64>)>>,
        halt: Arc<AtomicBool>,
        stopper: Option<Box<dyn FnOnce() + Send>>,
    },
}

pub fn get_target_bounds(target_id: Option<&str>) -> (u32, u32, i32, i32) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, RECT};
        use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
        use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

        if let Some(tid) = target_id {
            if let Some(hex) = tid.strip_prefix("window:0x") {
                if let Ok(hwnd_val) = usize::from_str_radix(hex, 16) {
                    let hwnd = HWND(hwnd_val as *mut _);
                    let mut r = RECT::default();
                    if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() {
                        let w = (r.right - r.left).max(100) as u32;
                        let h = (r.bottom - r.top).max(100) as u32;
                        return (w, h, r.left, r.top);
                    }
                }
            } else if let Some(idx_str) = tid.strip_prefix("display:") {
                // Resolve the origin for the monitor windows_capture ACTUALLY captures
                // (Monitor::from_index, the same order list_displays + GpuRecorder use),
                // correlating to its Win32 rect by GDI device name. Indexing EnumDisplayMonitors
                // directly used a DIFFERENT order, so display:N could take another monitor's origin
                // and shift every cursor point by the delta - the "cursor drawn in the wrong place"
                // regression. Matching device names keeps the video and the origin on one screen.
                if let Some(dev) = idx_str.parse::<usize>().ok()
                    .and_then(|i| windows_capture::monitor::Monitor::from_index(i).ok())
                    .and_then(|m| m.device_name().ok())
                {
                    struct MonCtx { want: Vec<u16>, bounds: Option<(u32, u32, i32, i32)> }
                    unsafe extern "system" fn enum_mon_cb(hmon: HMONITOR, _: HDC, _: *mut RECT, lparam: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::BOOL {
                        let ctx = &mut *(lparam.0 as *mut MonCtx);
                        let mut info = MONITORINFOEXW::default();
                        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                        if GetMonitorInfoW(hmon, &mut info.monitorInfo).as_bool() {
                            let n = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
                            if info.szDevice[..n] == ctx.want[..] {
                                let r = info.monitorInfo.rcMonitor;
                                ctx.bounds = Some(((r.right - r.left) as u32, (r.bottom - r.top) as u32, r.left, r.top));
                            }
                        }
                        windows::Win32::Foundation::BOOL(1)
                    }
                    let mut ctx = MonCtx { want: dev.encode_utf16().collect(), bounds: None };
                    unsafe { let _ = EnumDisplayMonitors(HDC::default(), None, Some(enum_mon_cb), windows::Win32::Foundation::LPARAM(&mut ctx as *mut _ as isize)); }
                    if let Some(b) = ctx.bounds { return b; }
                }
            }
        }
    }
    if let Ok(mon) = windows_capture::monitor::Monitor::primary() {
        let w = mon.width().unwrap_or(1920);
        let h = mon.height().unwrap_or(1080);
        return (w, h, 0, 0);
    }
    (1920, 1080, 0, 0)
}

/// Start the video pipeline writing `video_path`. GPU-native unless `legacy` is set; on a
/// GPU-encoder init error it logs and falls back to ffmpeg, so recording never simply fails.
/// Returns the sink plus the captured `(w, h, origin_x, origin_y)`.
pub fn start_video(legacy: bool, clock: Arc<dyn Clock>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, target_id: Option<&str>, video_path: &str) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let (_, _, ox, oy) = get_target_bounds(target_id);
    if !legacy {
        match GpuRecorder::start(clock.clone(), paused.clone(), fps, with_cursor, target_id, video_path) {
            Ok((r, w, h)) => return Ok((VideoSink::Gpu(r), w, h, ox, oy)),
            Err(e) => eprintln!("GPU encoder unavailable ({e}); falling back to ffmpeg"),
        }
    }
    start_ffmpeg(clock, stop, paused, fps, with_cursor, target_id, video_path, ox, oy)
}

/// The legacy ffmpeg path: WGC readback -> `RecordingSession::run` (VFR) -> `FfmpegFrameSink`.
fn start_ffmpeg(clock: Arc<dyn Clock>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, target_id: Option<&str>, video_path: &str, ox: i32, oy: i32) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let mut source = WgcFrameSource::for_target(clock, fps, with_cursor, target_id).map_err(|e| format!("screen capture init: {e}"))?;
    let (w, h) = source.dimensions();
    let sink = FfmpegFrameSink::new_vfr(video_path, w, h).map_err(|e| format!("video encoder spawn: {e}"))?;
    let halt = source.halt_handle();
    let stopper = source.take_stopper();
    let thread = std::thread::Builder::new().name("video".into()).spawn(move || {
        let mut session = RecordingSession::new(Box::new(source), Box::new(sink));
        session.run(&stop, &paused);
        let frame_ts = session.frame_timestamps().to_vec();
        let n = session.stop_and_finalize()?;
        Ok((n, frame_ts))
    }).map_err(|e| e.to_string())?;
    Ok((VideoSink::Ffmpeg { thread, halt, stopper }, w, h, ox, oy))
}

impl VideoSink {
    /// Stop the pipeline, finalize `video.mp4`, and return `(frame count, per-frame capture ms)`.
    pub fn stop_and_collect(self) -> Result<(u64, Vec<u64>), String> {
        match self {
            VideoSink::Gpu(r) => {
                let ts = r.stop()?;
                Ok((ts.len() as u64, ts))
            }
            VideoSink::Ffmpeg { thread, halt, stopper } => {
                halt.store(true, Ordering::SeqCst);
                if let Some(s) = stopper { s(); } // WM_QUIT -> WGC thread exits -> channel closes -> run() ends
                thread.join().map_err(|_| "video thread panicked".to_string())?.map_err(|e| e.to_string())
            }
        }
    }
}
