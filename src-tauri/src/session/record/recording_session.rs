use super::pause_clock::PauseClock;
use super::pause_totals::PauseTotals;
use crate::capture::frame_source::FrameSource;
use crate::domain::time::{Clock, Timestamp};
use crate::encode::frame_sink::FrameSink;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Recording,
    Stopped,
}

pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
    pause_clock: PauseClock,
    mismatched: bool,
}

impl RecordingSession {
    pub fn new(
        source: Box<dyn FrameSource>,
        sink: Box<dyn FrameSink>,
        totals: Arc<PauseTotals>,
    ) -> Self {
        Self {
            source,
            sink,
            state: SessionState::Recording,
            frames: 0,
            frame_ts: Vec::new(),
            pause_clock: PauseClock::new(totals),
            mismatched: false,
        }
    }
    pub fn state(&self) -> SessionState {
        self.state
    }
    pub fn frames_written(&self) -> u64 {
        self.frames
    }
    pub fn frame_timestamps(&self) -> &[u64] {
        &self.frame_ts
    }
    pub fn dimension_mismatch(&self) -> bool {
        self.mismatched
    }

    pub fn pump_once(&mut self) -> bool {
        match self.source.next_frame() {
            Some(mut frame) => {
                if let Some(tick) = self.pause_clock.tick(frame.ts.0, false) {
                    frame.ts = Timestamp(tick.sync_ms);
                    match self.sink.push(&frame) {
                        Ok(true) => {
                            self.frames += 1;
                            self.frame_ts.push(tick.sync_ms);
                        }
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
            if !self.pump_once() {
                break;
            }
        }
    }

    pub fn run(&mut self, stop: &AtomicBool, paused: &AtomicBool) {
        let mut was_paused = false;
        while !stop.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                was_paused = true;
                if self.source.next_frame().is_none() {
                    break;
                }
            } else {
                if std::mem::take(&mut was_paused) {
                    if let Err(e) = self.sink.split() {
                        eprintln!("segment split failed: {e}");
                    }
                }
                if !self.pump_once() {
                    break;
                }
            }
        }
    }

    pub fn run_paced(
        &mut self,
        stop: &AtomicBool,
        paused: &AtomicBool,
        clock: &dyn Clock,
        fps: u32,
    ) {
        crate::session::pacing::run_paced(
            &mut *self.source,
            &mut *self.sink,
            &mut self.frames,
            &mut self.frame_ts,
            stop,
            paused,
            clock,
            fps,
        );
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
