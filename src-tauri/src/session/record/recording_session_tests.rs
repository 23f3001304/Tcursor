// Split from recording_session.rs per repo convention (#[path] sibling test module) to
// stay under the 200-line file limit.
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
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
}

/// Mimics a dimension-mismatched frame (Task 5's `write_or_skip`): always reports the
/// frame as skipped (`Ok(false)`) rather than erroring.
struct SkippingSink;
impl FrameSink for SkippingSink {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> { Ok(false) }
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

#[test]
fn skipped_frame_is_not_counted_and_pushes_no_timestamp() {
    // A skipped frame (Ok(false), e.g. a mid-record dimension mismatch) must not
    // inflate `frames` or `frame_ts` - sync.json would otherwise gain a phantom
    // timestamp for a frame that was never actually written to the video.
    let mut session = RecordingSession::new(
        Box::new(FakeFrameSource::new(vec![frame(0), frame(33)])),
        Box::new(SkippingSink),
    );
    assert!(session.pump_once());
    assert!(session.pump_once());
    assert_eq!(session.frames_written(), 0);
    assert!(session.frame_timestamps().is_empty());
}
