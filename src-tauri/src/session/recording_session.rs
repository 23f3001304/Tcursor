use std::sync::atomic::{AtomicBool, Ordering};
use crate::capture::frame_source::FrameSource;
use crate::encode::frame_sink::FrameSink;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState { Idle, Recording, Stopped }

pub struct RecordingSession {
    source: Box<dyn FrameSource>,
    sink: Box<dyn FrameSink>,
    state: SessionState,
    frames: u64,
    frame_ts: Vec<u64>,
}

impl RecordingSession {
    pub fn new(source: Box<dyn FrameSource>, sink: Box<dyn FrameSink>) -> Self {
        Self { source, sink, state: SessionState::Recording, frames: 0, frame_ts: Vec::new() }
    }
    pub fn state(&self) -> SessionState { self.state }
    pub fn frames_written(&self) -> u64 { self.frames }
    /// Capture timestamps (ms) of each successfully encoded frame, in order.
    pub fn frame_timestamps(&self) -> &[u64] { &self.frame_ts }

    /// Pull one frame and write it. Returns false when the source is exhausted.
    pub fn pump_once(&mut self) -> bool {
        match self.source.next_frame() {
            Some(frame) => {
                match self.sink.push(&frame) {
                    Ok(()) => { self.frames += 1; self.frame_ts.push(frame.ts.0); }
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
    /// discarded instead of encoded, so paused time is excluded from the recording.
    pub fn run(&mut self, stop: &AtomicBool, paused: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            if paused.load(Ordering::SeqCst) {
                if self.source.next_frame().is_none() { break; }
            } else if !self.pump_once() {
                break;
            }
        }
    }

    pub fn stop_and_finalize(mut self) -> std::io::Result<u64> {
        self.state = SessionState::Stopped;
        self.sink.finish()?;
        Ok(self.frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use crate::capture::frame::Frame;
    use crate::capture::frame_source::FakeFrameSource;
    use crate::domain::time::Timestamp;
    use crate::encode::frame_sink::{FakeFrameSink, FrameSink};

    fn frame(ts: u64) -> Frame {
        Frame { width: 2, height: 2, bgra: vec![0; 2*2*4], ts: Timestamp(ts) }
    }

    struct FailingSink;
    impl FrameSink for FailingSink {
        fn push(&mut self, _f: &Frame) -> std::io::Result<()> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
        }
        fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
    }

    #[test]
    fn pumps_all_frames_then_reports_exhausted() {
        let src = Box::new(FakeFrameSource::new(vec![frame(0), frame(33), frame(66)]));
        let sink = Box::new(FakeFrameSink::default());
        let mut session = RecordingSession::new(src, sink);
        assert!(matches!(session.state(), SessionState::Recording));
        assert!(session.pump_once());      // frame 0
        assert!(session.pump_once());      // frame 1
        assert!(session.pump_once());      // frame 2
        assert!(!session.pump_once());     // exhausted
        assert_eq!(session.frames_written(), 3);
    }

    #[test]
    fn run_until_stopped_halts_on_flag() {
        let frames: Vec<Frame> = (0..1000).map(|i| frame(i * 33)).collect();
        let src = Box::new(FakeFrameSource::new(frames));
        let sink = Box::new(FakeFrameSink::default());
        let mut session = RecordingSession::new(src, sink);
        let stop = AtomicBool::new(true); // already stopped -> writes zero
        session.run_until_stopped(&stop);
        assert_eq!(session.frames_written(), 0);
    }

    #[test]
    fn finalize_returns_count_and_marks_stopped() {
        let src = Box::new(FakeFrameSource::new(vec![frame(0)]));
        let sink = Box::new(FakeFrameSink::default());
        let mut session = RecordingSession::new(src, sink);
        session.pump_once();
        let n = session.stop_and_finalize().unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn frames_written_counts_only_successful_pushes() {
        let src = Box::new(FakeFrameSource::new(vec![frame(0), frame(33)]));
        let sink = Box::new(FailingSink);
        let mut session = RecordingSession::new(src, sink);
        assert!(session.pump_once());   // pulled; push failed
        assert!(session.pump_once());   // pulled; push failed
        assert!(!session.pump_once());  // exhausted
        assert_eq!(session.frames_written(), 0);
    }

    #[test]
    fn run_encodes_when_not_paused() {
        let src = Box::new(FakeFrameSource::new(vec![frame(0), frame(33)]));
        let sink = Box::new(FakeFrameSink::default());
        let mut session = RecordingSession::new(src, sink);
        let stop = AtomicBool::new(false);
        let paused = AtomicBool::new(false);
        session.run(&stop, &paused);
        assert_eq!(session.frames_written(), 2);
    }

    #[test]
    fn run_discards_frames_while_paused() {
        let src = Box::new(FakeFrameSource::new(vec![frame(0), frame(33)]));
        let sink = Box::new(FakeFrameSink::default());
        let mut session = RecordingSession::new(src, sink);
        let stop = AtomicBool::new(false);
        let paused = AtomicBool::new(true);
        session.run(&stop, &paused);
        assert_eq!(session.frames_written(), 0);
    }
}
