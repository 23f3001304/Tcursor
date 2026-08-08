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
                    Ok(()) => { self.frames += 1; self.frame_ts.push(ts); }
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
mod tests {
    use super::*;
    use crate::capture::frame::Frame;
    use crate::capture::frame_source::FakeFrameSource;
    use crate::domain::time::Timestamp;
    use crate::encode::frame_sink::FakeFrameSink;

    fn frame(ts: u64) -> Frame {
        Frame { width: 2, height: 2, bgra: vec![0; 2 * 2 * 4], ts: Timestamp(ts) }
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
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(0), frame(33), frame(66)])),
            Box::new(FakeFrameSink::default()),
        );
        assert!(session.pump_once()); assert!(session.pump_once()); assert!(session.pump_once());
        assert!(!session.pump_once());
        assert_eq!(session.frames_written(), 3);
    }

    #[test]
    fn run_until_stopped_halts_on_flag() {
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new((0..10).map(|i| frame(i * 33)).collect())),
            Box::new(FakeFrameSink::default()),
        );
        session.run_until_stopped(&AtomicBool::new(true));
        assert_eq!(session.frames_written(), 0);
    }

    #[test]
    fn finalize_returns_count_and_marks_stopped() {
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(0)])),
            Box::new(FakeFrameSink::default()),
        );
        session.pump_once();
        assert_eq!(session.stop_and_finalize().unwrap(), 1);
    }

    #[test]
    fn run_encodes_when_not_paused() {
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(0), frame(33)])),
            Box::new(FakeFrameSink::default()),
        );
        session.run(&AtomicBool::new(false), &AtomicBool::new(false));
        assert_eq!(session.frames_written(), 2);
    }

    #[test]
    fn run_discards_frames_while_paused() {
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(0), frame(33)])),
            Box::new(FakeFrameSink::default()),
        );
        session.run(&AtomicBool::new(false), &AtomicBool::new(true));
        assert_eq!(session.frames_written(), 0);
    }

    #[test]
    fn run_shifts_post_pause_timestamps_by_the_paused_span() {
        // push@1000 (unpaused), then two frames arrive at 1000 and 3000 while
        // paused=true throughout this run() call - both discarded, but their
        // timestamps mark the 2000ms pause span via pause_clock. The next real
        // pump (3100, injected after swapping the exhausted source) is shifted
        // to 1100: run()'s discard branch is correctly wired to PauseClock.
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(1000)])),
            Box::new(FakeFrameSink::default()),
        );
        assert!(session.pump_once());
        session.source = Box::new(FakeFrameSource::new(vec![frame(1000), frame(3000)]));
        session.run(&AtomicBool::new(false), &AtomicBool::new(true));
        assert_eq!(session.frames_written(), 1); // still just the first, unpaused push
        session.source = Box::new(FakeFrameSource::new(vec![frame(3100)]));
        assert!(session.pump_once());
        assert_eq!(session.frame_timestamps(), &[1000, 1100]);
    }

    #[test]
    fn frames_written_counts_only_successful_pushes() {
        let mut session = RecordingSession::new(
            Box::new(FakeFrameSource::new(vec![frame(0), frame(33)])),
            Box::new(FailingSink),
        );
        assert!(session.pump_once());
        assert!(session.pump_once());
        assert!(!session.pump_once());
        assert_eq!(session.frames_written(), 0);
    }
}
