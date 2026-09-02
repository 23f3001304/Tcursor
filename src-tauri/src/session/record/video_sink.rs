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
use crate::encode::vfr_segments::VfrSegments;
use crate::session::record::gpu_record::{GpuRecorder, GpuStart};
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::recording_session::RecordingSession;
use crate::session::record::{Notify, CAPTURE_CLOSED, DISPLAY_CHANGED};

/// A live recording video pipeline.
pub enum VideoSink {
    /// GPU-native: the windows-capture Media Foundation encoder, hardware-encoding on the GPU.
    Gpu(GpuRecorder),
    /// Legacy fallback: WGC readback -> `RecordingSession` -> ffmpeg rawvideo pipe, on a thread.
    Ffmpeg {
        thread: JoinHandle<VideoStopped>,
        halt: Arc<AtomicBool>,
        stopper: Option<Box<dyn FnOnce() + Send>>,
    },
}

/// What a stopped video pipeline leaves behind. `error` is reported separately from the
/// timestamps ON PURPOSE: a finalize failure used to propagate before `sync.json`, the
/// `.tcursor` manifest and the recents entry were written, so an encoder/mux tail failure
/// (disk full on a long take) left a folder that could not even be opened. The caller writes
/// those from `frame_ts` first and surfaces `error` after.
pub struct VideoStopped {
    pub frames: u64,
    pub frame_ts: Vec<u64>,
    pub error: Option<String>,
}

/// Everything both capture paths need to start, beside the target and the output path.
pub struct VideoStart {
    /// Force the legacy ffmpeg pipeline (the HUD's compatibility toggle).
    pub legacy: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    /// The exact-span pause ledger both the video timestamps and every input stream subtract.
    pub totals: Arc<PauseTotals>,
    /// Called if the OS ends the capture on its own, so the app can run its normal Stop.
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
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

/// Start the video pipeline writing `video_path`. GPU-native unless `cfg.legacy` is set; on a
/// GPU-encoder init error it logs and falls back to ffmpeg, so recording never simply fails.
/// Returns the sink plus the captured `(w, h, origin_x, origin_y)`.
pub fn start_video(cfg: VideoStart, target_id: Option<&str>, video_path: &str) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let (_, _, ox, oy) = get_target_bounds(target_id);
    if !cfg.legacy {
        let gpu = GpuStart {
            clock: cfg.clock.clone(), paused: cfg.paused.clone(), totals: cfg.totals.clone(),
            ended: cfg.ended.clone(), fps: cfg.fps, with_cursor: cfg.with_cursor,
        };
        match GpuRecorder::start(gpu, target_id, video_path) {
            Ok((r, w, h)) => return Ok((VideoSink::Gpu(r), w, h, ox, oy)),
            Err(e) => eprintln!("GPU encoder unavailable ({e}); falling back to ffmpeg"),
        }
    }
    start_ffmpeg(cfg, target_id, video_path, ox, oy)
}

/// The legacy ffmpeg path: WGC readback -> `RecordingSession::run` (VFR) -> `VfrSegments`.
fn start_ffmpeg(cfg: VideoStart, target_id: Option<&str>, video_path: &str, ox: i32, oy: i32) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let mut source = WgcFrameSource::for_target(cfg.clock, cfg.fps, cfg.with_cursor, target_id).map_err(|e| format!("screen capture init: {e}"))?;
    let (w, h) = source.dimensions();
    let sink = VfrSegments::new(video_path, w, h).map_err(|e| format!("video encoder spawn: {e}"))?;
    let halt = source.halt_handle();
    let stopper = source.take_stopper();
    let (stop, paused, totals, ended) = (cfg.stop, cfg.paused, cfg.totals, cfg.ended);
    let thread = std::thread::Builder::new().name("video".into()).spawn(move || {
        let mut session = RecordingSession::new(Box::new(source), Box::new(sink), totals);
        session.run(&stop, &paused);
        // `run` returned on its own (not via the `stop` flag a user Stop sets - that reaches
        // the same loop exit through the halt flag and the WM_QUIT stopper, and must not be
        // reported as an early end). Two different things can cause that: the frame source ran
        // dry because WGC closed the capture (recorded window closed, display unplugged), or
        // `pump_once` latched a dimension mismatch (H1: window resize/maximize, display
        // resolution/rotation change, dock/undock) and stopped pumping on purpose. Without this
        // either way the video is quietly sealed here while the HUD keeps counting and the mic
        // keeps taking narration.
        if !stop.load(Ordering::SeqCst) {
            if session.dimension_mismatch() { ended(DISPLAY_CHANGED); } else { ended(CAPTURE_CLOSED); }
        }
        let (frames, frame_ts) = (session.frames_written(), session.frame_timestamps().to_vec());
        let error = session.stop_and_finalize().err().map(|e| e.to_string());
        VideoStopped { frames, frame_ts, error }
    }).map_err(|e| e.to_string())?;
    Ok((VideoSink::Ffmpeg { thread, halt, stopper }, w, h, ox, oy))
}

impl VideoSink {
    /// Stop the pipeline and finalize `video.mp4`. A finalize failure comes back inside
    /// `VideoStopped::error` rather than replacing the result, so the caller can still write
    /// `sync.json` for the frames that did make it.
    pub fn stop_and_collect(self) -> VideoStopped {
        match self {
            VideoSink::Gpu(r) => r.stop(),
            VideoSink::Ffmpeg { thread, halt, stopper } => {
                halt.store(true, Ordering::SeqCst);
                if let Some(s) = stopper { s(); } // WM_QUIT -> WGC thread exits -> channel closes -> run() ends
                thread.join().unwrap_or(VideoStopped {
                    frames: 0, frame_ts: Vec::new(), error: Some("video thread panicked".into()),
                })
            }
        }
    }
}
