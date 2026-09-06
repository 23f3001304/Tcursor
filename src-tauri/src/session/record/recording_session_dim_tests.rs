// Split from recording_session_tests.rs per repo convention (#[path] sibling test module) to
// stay under the 200-line file limit. Task 5 (H1): a mid-record dimension change (window
// resize/maximize, display resolution/rotation change, dock/undock) must end the take
// gracefully instead of the sink skipping every frame after it forever.
use super::*;
use std::sync::atomic::AtomicUsize;
use crate::capture::frame::Frame;
use crate::capture::frame_source::FakeFrameSource;
use crate::domain::time::Timestamp;

fn frame(ts: u64) -> Frame {
    Frame { width: 2, height: 2, bgra: vec![0; 2 * 2 * 4], ts: Timestamp(ts) }
}

fn session(frames: Vec<Frame>, sink: Box<dyn FrameSink>) -> RecordingSession {
    RecordingSession::new(Box::new(FakeFrameSource::new(frames)), sink, Arc::new(PauseTotals::new()))
}

/// Mimics a dimension-mismatched frame (`FfmpegFrameSink`'s `write_or_skip`): always reports
/// the frame as skipped (`Ok(false)`) rather than erroring.
struct SkippingSink;
impl FrameSink for SkippingSink {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> { Ok(false) }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
}

/// A dimension-mismatched frame must NOT end the take. A browser tab switch (bookmarks bar,
/// swapchain recreation) or a window resize changes the capture surface constantly, and ending
/// the recording there made recording a browsing session impossible. `pump_once` keeps reporting
/// "keep going"; the frame is simply neither counted nor timestamped, so `sync.json` never gains
/// a phantom timestamp for a frame the video does not contain.
#[test]
fn a_dimension_mismatch_keeps_the_take_recording() {
    let mut s = session(vec![frame(0), frame(33)], Box::new(SkippingSink));
    assert!(s.pump_once(), "a mismatched frame must not end the take - a resize is not a failure");
    assert_eq!(s.frames_written(), 0);
    assert!(s.frame_timestamps().is_empty());

}

/// Counts `push` calls, always reporting a mismatch - the seam-level proof that `run`'s loop
/// keeps feeding the sink instead of ending the take on the first mismatch.
struct CountingSkipSink { calls: Arc<AtomicUsize> }
impl FrameSink for CountingSkipSink {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(false)
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
}

#[test]
fn run_keeps_pumping_through_mismatched_frames() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut s = session(vec![frame(0), frame(33), frame(66)], Box::new(CountingSkipSink { calls: calls.clone() }));
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert!(calls.load(Ordering::SeqCst) > 1, "the loop must keep feeding the sink through mismatches, not end the take");
    assert_eq!(s.frames_written(), 0);
    assert!(s.frame_timestamps().is_empty());

}

/// A sink that writes the first `n` frames, then reports every later one as mismatched - the
/// span before a mid-record resize.
struct MismatchAfterN { n: usize, seen: usize }
impl FrameSink for MismatchAfterN {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
        self.seen += 1;
        Ok(self.seen <= self.n)
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
}

/// The prior span - everything recorded before the dimension changed - must survive intact:
/// counted, timestamped, and still cleanly finalizable, exactly as if Stop had been pressed
/// right there.
#[test]
fn a_mismatch_after_some_frames_still_finalizes_the_prior_span_cleanly() {
    let mut s = session(vec![frame(0), frame(33), frame(66), frame(99)], Box::new(MismatchAfterN { n: 2, seen: 0 }));
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert_eq!(s.frames_written(), 2);
    assert_eq!(s.frame_timestamps(), &[0, 33]);

    assert_eq!(s.stop_and_finalize().unwrap(), 2, "the prior span must still finalize cleanly");
}
