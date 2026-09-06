use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::capture::frame_source::FrameSource;
use crate::domain::time::{Clock, Timestamp};
use crate::encode::frame_sink::FrameSink;
use super::pause_clock::PauseClock;
use super::pause_totals::PauseTotals;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState { Idle, Recording, Stopped }

pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
    pause_clock: PauseClock,
    /// Set the first time `sink.push` reports a dimension-mismatched frame (`Ok(false)`) - a
    /// mid-record window resize/maximize or display resolution/rotation change. Read by
    /// `video_sink::start_ffmpeg` after `run` returns, to tell that apart from the source
    /// simply running dry (the OS closing the capture), which gets a different HUD reason.
    mismatched: bool,
}

impl RecordingSession {
    pub fn new(source: Box<dyn FrameSource>, sink: Box<dyn FrameSink>, totals: Arc<PauseTotals>) -> Self {
        Self { source, sink, state: SessionState::Recording, frames: 0, frame_ts: Vec::new(), pause_clock: PauseClock::new(totals), mismatched: false }
    }
    pub fn state(&self) -> SessionState { self.state }
    pub fn frames_written(&self) -> u64 { self.frames }
    /// Capture timestamps (ms) of each successfully encoded frame, in order.
    pub fn frame_timestamps(&self) -> &[u64] { &self.frame_ts }
    /// True once a dimension-mismatched frame has ended the take early (see `mismatched`).
    pub fn dimension_mismatch(&self) -> bool { self.mismatched }

    /// Pull one frame and write it. Returns false when the source is exhausted OR the sink just
    /// reported the FIRST dimension mismatch - the latter also latches `mismatched`, which is
    /// what lets the caller tell the two "stop pumping" reasons apart. The recorded timestamp
    /// comes from `pause_clock`, so it excludes every paused span; a frame the clock declines
    /// (it would not advance the compressed timeline) is dropped without being counted or
    /// timestamped, exactly like a mismatched one.
    pub fn pump_once(&mut self) -> bool {
        match self.source.next_frame() {
            Some(mut frame) => {
                if let Some(tick) = self.pause_clock.tick(frame.ts.0, false) {
                    // Hand the sink the frame on the RECORDING clock rather than the raw
                    // capture clock. `VfrSegments` needs each part's first pause-compressed
                    // timestamp to compute its concat offsets; every other sink ignores `ts`.
                    frame.ts = Timestamp(tick.sync_ms);
                    match self.sink.push(&frame) {
                        Ok(true) => { self.frames += 1; self.frame_ts.push(tick.sync_ms); }
                        // A dimension mismatch (window resize/maximize, display
                        // resolution/rotation change, or a browser tab switch toggling the
                        // bookmarks bar). NOT fatal: this frame is neither counted nor
                        // timestamped, but the take keeps recording. Ending it here made
                        // recording a browsing session impossible.
                        Ok(false) => {}
                        Err(e) => eprintln!("frame sink push failed: {e}"),
                    }
                }
                true
            }
            None => false,
        }
    }

    pub fn run_until_stopped(&mut self, stop: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            if !self.pump_once() { break; }
        }
    }

    /// Like `run_until_stopped`, but while `paused` is set frames are pulled and discarded
    /// instead of encoded (the paused span itself is measured by the ledger `pause_clock`
    /// reads, not by these arrivals). Each resume also `split`s the sink, which is how the
    /// wall-clock-stamped ffmpeg sink keeps the paused span out of the video's own PTS.
    pub fn run(&mut self, stop: &AtomicBool, paused: &AtomicBool) {
        let mut was_paused = false;
        while !stop.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                was_paused = true;
                if self.source.next_frame().is_none() { break; }
            } else {
                if std::mem::take(&mut was_paused) {
                    if let Err(e) = self.sink.split() { eprintln!("segment split failed: {e}"); }
                }
                if !self.pump_once() { break; }
            }
        }
    }

    /// Constant-frame-rate (game-mode) capture: a steady `fps` stream with uniform
    /// timestamps, so variable-FPS sources record smoothly. Delegates to `session::pacing`.
    pub fn run_paced(&mut self, stop: &AtomicBool, paused: &AtomicBool, clock: &dyn Clock, fps: u32) {
        crate::session::pacing::run_paced(&mut *self.source, &mut *self.sink, &mut self.frames, &mut self.frame_ts, stop, paused, clock, fps);
    }

    pub fn stop_and_finalize(mut self) -> std::io::Result<u64> {
        self.state = SessionState::Stopped;
        self.sink.finish()?;
        Ok(self.frames)
    }
}

#[cfg(test)]
#[path = "recording_session_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "recording_session_dim_tests.rs"]
mod dim_tests;
