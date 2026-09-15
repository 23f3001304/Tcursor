pub mod gpu;
pub mod legacy;
pub mod target;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::domain::time::Clock;
use crate::ports::capture::{
    CaptureGeometry, CapturePort, CaptureRequest, CaptureTarget, FirstFrameSize, TargetId,
    VideoSink as VideoSinkPort,
};
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::video_sink::VideoStopped;
use crate::session::record::Notify;
use gpu::frames::{FrameTimes, SizeHook};
use gpu::record::{GpuRecorder, GpuStart};

const NO_GPU: &str = "switching needs the GPU encoder; turn the compatibility encoder off";

pub enum VideoSink {
    Gpu(GpuRecorder),
    Ffmpeg {
        thread: JoinHandle<VideoStopped>,
        halt: Arc<AtomicBool>,
        stopper: Option<Box<dyn FnOnce() + Send>>,
    },
    Dead(FrameTimes),
}

pub struct VideoStart {
    pub legacy: bool,
    pub clock: Arc<dyn Clock>,
    pub stop: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub fps: u32,
    pub with_cursor: bool,
}

pub struct Win32Capture;

pub struct Win32Sink {
    sink: VideoSink,
    stop: Arc<AtomicBool>,
}

fn start_video(
    cfg: VideoStart,
    target_id: Option<&str>,
    video_path: &str,
) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let origin = target::get_target_bounds(target_id);
    let (ox, oy) = (origin.origin_x, origin.origin_y);
    if !cfg.legacy {
        let gpu = GpuStart {
            clock: cfg.clock.clone(),
            paused: cfg.paused.clone(),
            totals: cfg.totals.clone(),
            ended: cfg.ended.clone(),
            fps: cfg.fps,
            with_cursor: cfg.with_cursor,
        };
        match GpuRecorder::start(gpu, target_id, video_path) {
            Ok((r, w, h)) => return Ok((VideoSink::Gpu(r), w, h, ox, oy)),
            Err(e) => eprintln!("GPU encoder unavailable ({e}); falling back to ffmpeg"),
        }
    }
    legacy::start_ffmpeg(cfg, target_id, video_path, ox, oy)
}

impl VideoSink {
    fn switch(
        &mut self,
        cfg: VideoStart,
        target_id: &str,
        on_size: SizeHook,
    ) -> Result<(), String> {
        let VideoSink::Gpu(live) = self else {
            return Err(NO_GPU.into());
        };
        let dead = VideoSink::Dead(live.frame_times());
        let gpu = GpuStart {
            clock: cfg.clock,
            paused: cfg.paused,
            totals: cfg.totals,
            ended: cfg.ended,
            fps: cfg.fps,
            with_cursor: cfg.with_cursor,
        };
        let VideoSink::Gpu(rec) = std::mem::replace(self, dead) else {
            return Err(NO_GPU.into());
        };
        match rec.restart(gpu, Some(target_id), on_size) {
            Ok((r, _, _)) => {
                *self = VideoSink::Gpu(r);
                Ok(())
            }
            Err(e) => Err(format!("display switch: {e}")),
        }
    }

    fn stop_and_collect(self) -> VideoStopped {
        match self {
            VideoSink::Gpu(r) => r.stop(),
            VideoSink::Dead(ts) => {
                let frame_ts = ts.lock().unwrap_or_else(|e| e.into_inner()).clone();
                VideoStopped {
                    frames: frame_ts.len() as u64,
                    frame_ts,
                    error: Some(
                        "a display switch stopped the capture and the replacement did not start"
                            .into(),
                    ),
                }
            }
            VideoSink::Ffmpeg {
                thread,
                halt,
                stopper,
            } => {
                halt.store(true, Ordering::SeqCst);
                if let Some(s) = stopper {
                    s();
                }
                thread.join().unwrap_or(VideoStopped {
                    frames: 0,
                    frame_ts: Vec::new(),
                    error: Some("video thread panicked".into()),
                })
            }
        }
    }
}

fn video_start(req: &CaptureRequest) -> VideoStart {
    VideoStart {
        legacy: req.prefer_compatibility,
        clock: req.clock.clone(),
        stop: req.stop.clone(),
        paused: req.paused.clone(),
        totals: req.totals.clone(),
        ended: req.ended.clone(),
        fps: req.fps,
        with_cursor: req.with_cursor,
    }
}

impl VideoSinkPort for Win32Sink {
    fn switch(
        &mut self,
        req: CaptureRequest,
        on_first_frame: FirstFrameSize,
    ) -> Result<(), String> {
        let cfg = video_start(&req);
        self.sink
            .switch(cfg, &req.target.to_string(), on_first_frame)
    }

    fn stop(self: Box<Self>) -> VideoStopped {
        let Win32Sink { sink, stop } = *self;
        stop.store(true, Ordering::SeqCst);
        sink.stop_and_collect()
    }

    fn supports_switch(&self) -> bool {
        matches!(self.sink, VideoSink::Gpu(_))
    }
}

impl CapturePort for Win32Capture {
    fn list_targets(&self) -> Vec<CaptureTarget> {
        target::list_targets()
    }

    fn bounds(&self, target: &TargetId) -> CaptureGeometry {
        target::get_target_bounds(target.as_arg().as_deref())
    }

    fn start(
        &self,
        req: CaptureRequest,
    ) -> Result<(Box<dyn VideoSinkPort>, CaptureGeometry), String> {
        let stop = req.stop.clone();
        let cfg = video_start(&req);
        let path = req.output.to_string_lossy().into_owned();
        let (sink, w, h, origin_x, origin_y) =
            start_video(cfg, req.target.as_arg().as_deref(), &path)?;
        let geometry = CaptureGeometry {
            w,
            h,
            origin_x,
            origin_y,
        };
        Ok((Box::new(Win32Sink { sink, stop }), geometry))
    }
}
