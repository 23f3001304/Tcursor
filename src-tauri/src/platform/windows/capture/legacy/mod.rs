pub mod wgc_source;

use std::sync::atomic::Ordering;

use super::{VideoSink, VideoStart};
use crate::capture::frame_source::FrameSource;
use crate::encode::vfr_segments::VfrSegments;
use crate::session::record::recording_session::RecordingSession;
use crate::session::record::video_sink::VideoStopped;
use crate::session::record::{CAPTURE_CLOSED, DISPLAY_CHANGED};
use wgc_source::WgcFrameSource;

pub(super) fn start_ffmpeg(
    cfg: VideoStart,
    target_id: Option<&str>,
    video_path: &str,
    ox: i32,
    oy: i32,
) -> Result<(VideoSink, u32, u32, i32, i32), String> {
    let mut source = WgcFrameSource::for_target(cfg.clock, cfg.fps, cfg.with_cursor, target_id)
        .map_err(|e| format!("screen capture init: {e}"))?;
    let (w, h) = source.dimensions();
    let sink =
        VfrSegments::new(video_path, w, h).map_err(|e| format!("video encoder spawn: {e}"))?;
    let halt = source.halt_handle();
    let stopper = source.take_stopper();
    let (stop, paused, totals, ended) = (cfg.stop, cfg.paused, cfg.totals, cfg.ended);
    let thread = std::thread::Builder::new()
        .name("video".into())
        .spawn(move || {
            let mut session = RecordingSession::new(Box::new(source), Box::new(sink), totals);
            session.run(&stop, &paused);
            if !stop.load(Ordering::SeqCst) {
                if session.dimension_mismatch() {
                    ended(DISPLAY_CHANGED);
                } else {
                    ended(CAPTURE_CLOSED);
                }
            }
            let (frames, frame_ts) = (
                session.frames_written(),
                session.frame_timestamps().to_vec(),
            );
            let error = session.stop_and_finalize().err().map(|e| e.to_string());
            VideoStopped {
                frames,
                frame_ts,
                error,
            }
        })
        .map_err(|e| e.to_string())?;
    Ok((
        VideoSink::Ffmpeg {
            thread,
            halt,
            stopper,
        },
        w,
        h,
        ox,
        oy,
    ))
}
