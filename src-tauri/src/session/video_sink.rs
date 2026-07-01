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
use crate::session::gpu_record::GpuRecorder;
use crate::session::recording_session::RecordingSession;

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

/// Start the video pipeline writing `video_path`. GPU-native unless `legacy` is set; on a
/// GPU-encoder init error it logs and falls back to ffmpeg, so recording never simply fails.
/// Returns the sink plus the captured `(w, h)`.
pub fn start_video(legacy: bool, clock: Arc<dyn Clock>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, video_path: &str) -> Result<(VideoSink, u32, u32), String> {
    if !legacy {
        match GpuRecorder::start(clock.clone(), paused.clone(), fps, with_cursor, video_path) {
            Ok((r, w, h)) => return Ok((VideoSink::Gpu(r), w, h)),
            Err(e) => eprintln!("GPU encoder unavailable ({e}); falling back to ffmpeg"),
        }
    }
    start_ffmpeg(clock, stop, paused, fps, with_cursor, video_path)
}

/// The legacy ffmpeg path: WGC readback -> `RecordingSession::run` (VFR) -> `FfmpegFrameSink`.
fn start_ffmpeg(clock: Arc<dyn Clock>, stop: Arc<AtomicBool>, paused: Arc<AtomicBool>, fps: u32, with_cursor: bool, video_path: &str) -> Result<(VideoSink, u32, u32), String> {
    let mut source = WgcFrameSource::for_primary_display(clock, fps, with_cursor).map_err(|e| format!("screen capture init: {e}"))?;
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
    Ok((VideoSink::Ffmpeg { thread, halt, stopper }, w, h))
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
