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
use crate::session::record::gpu_frames::{FrameTimes, SizeHook};
use crate::session::record::gpu_record::{GpuRecorder, GpuStart};
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::recording_session::RecordingSession;
use crate::session::record::target_bounds::get_target_bounds;
use crate::session::record::{Notify, CAPTURE_CLOSED, DISPLAY_CHANGED};

/// What `VideoSink::switch` refuses with when the take is on the legacy pipeline. Its rawvideo
/// pipe is sized once at start and its `RecordingSession` ends the take on the first mismatched
/// frame, so there is nothing to restart into.
const NO_GPU: &str = "switching needs the GPU encoder; turn the compatibility encoder off";

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
    /// A display switch stopped the capture and the replacement never started. Nothing is
    /// recording and `video.mp4` is finalized, but the frame timestamps of what WAS recorded are
    /// still here (the same `Arc` the dead capture pushed into), so the stop path can write a
    /// truthful `sync.json` instead of losing the take.
    Dead(FrameTimes),
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
    /// Move a running capture to another display or window mid-take, keeping the encoder - so
    /// `video.mp4` stays one stream at one size and the editor never learns a second screen
    /// existed. `on_size` is told the replacement capture's first frame size (`switch_display`
    /// writes it into the switch record); the target's rectangle is `get_target_bounds`' business.
    pub fn switch(&mut self, cfg: VideoStart, target_id: &str, on_size: SizeHook) -> Result<(), String> {
        let VideoSink::Gpu(live) = self else { return Err(NO_GPU.into()) };
        // `restart` consumes the recorder, so something has to stand in its place: `Dead` holds
        // the shared timestamps, which keep filling until the old capture's thread exits.
        let dead = VideoSink::Dead(live.frame_times());
        let gpu = GpuStart {
            clock: cfg.clock, paused: cfg.paused, totals: cfg.totals,
            ended: cfg.ended, fps: cfg.fps, with_cursor: cfg.with_cursor,
        };
        let VideoSink::Gpu(rec) = std::mem::replace(self, dead) else { return Err(NO_GPU.into()) };
        match rec.restart(gpu, Some(target_id), on_size) {
            Ok((r, _, _)) => { *self = VideoSink::Gpu(r); Ok(()) }
            Err(e) => Err(format!("display switch: {e}")),
        }
    }

    /// Stop the pipeline and finalize `video.mp4`. A finalize failure comes back inside
    /// `VideoStopped::error` rather than replacing the result, so the caller can still write
    /// `sync.json` for the frames that did make it.
    pub fn stop_and_collect(self) -> VideoStopped {
        match self {
            VideoSink::Gpu(r) => r.stop(),
            VideoSink::Dead(ts) => {
                let frame_ts = ts.lock().unwrap_or_else(|e| e.into_inner()).clone();
                VideoStopped {
                    frames: frame_ts.len() as u64, frame_ts,
                    error: Some("a display switch stopped the capture and the replacement did not start".into()),
                }
            }
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
