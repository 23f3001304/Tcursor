use std::sync::atomic::{AtomicBool, Ordering};
use crate::capture::frame_source::FrameSource;
use crate::domain::time::Clock;
use crate::encode::frame_sink::FrameSink;
use super::pause_clock::PauseClock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState { Idle, Recording, Stopped }

pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
    pause_clock: PauseClock,
}

impl RecordingSession {
    pub fn new(source: Box<dyn FrameSource>, sink: Box<dyn FrameSink>) -> Self {
        Self { source, sink, state: SessionState::Recording, frames: 0, frame_ts: Vec::new(), pause_clock: PauseClock::new() }
    }
    pub fn state(&self) -> SessionState { self.state }
    pub fn frames_written(&self) -> u64 { self.frames }
    /// Capture timestamps (ms) of each successfully encoded frame, in order.
    pub fn frame_timestamps(&self) -> &[u64] { &self.frame_ts }

    /// Pull one frame and write it. Returns false when the source is exhausted. The
    /// recorded timestamp is shifted by `pause_clock` so it excludes any prior paused
    /// span (a no-op outside `run`, which is the only caller that ever pauses).
    pub fn pump_once(&mut self) -> bool {
        match self.source.next_frame() {
            Some(frame) => {
                let ts = self.pause_clock.observe(frame.ts.0, false).unwrap_or(frame.ts.0);
                match self.sink.push(&frame) {
                    Ok(true) => { self.frames += 1; self.frame_ts.push(ts); }
                    Ok(false) => {} // dimension-mismatched frame, skipped by the sink - do not count or timestamp it
                    Err(e) => eprintln!("frame sink push failed: {e}"),
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

    /// Like `run_until_stopped`, but while `paused` is set frames are pulled and
    /// discarded instead of encoded, and their timestamps feed `pause_clock` so
    /// paused time is excluded from every timestamp recorded after resume.
    pub fn run(&mut self, stop: &AtomicBool, paused: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                match self.source.next_frame() {
                    Some(f) => { self.pause_clock.observe(f.ts.0, true); }
                    None => break,
                }
            } else if !self.pump_once() {
                break;
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
