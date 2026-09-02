// Split from recording_session.rs per repo convention (#[path] sibling test module) to
// stay under the 200-line file limit.
use super::*;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use crate::capture::frame::Frame;
use crate::capture::frame_source::FakeFrameSource;
use crate::domain::time::Timestamp;
use crate::encode::frame_sink::FakeFrameSink;

fn frame(ts: u64) -> Frame {
    Frame { width: 2, height: 2, bgra: vec![0; 2 * 2 * 4], ts: Timestamp(ts) }
}

/// A session over the given frames with its own fresh (never-paused) ledger.
fn session(frames: Vec<Frame>, sink: Box<dyn FrameSink>) -> RecordingSession {
    RecordingSession::new(Box::new(FakeFrameSource::new(frames)), sink, Arc::new(PauseTotals::new()))
}

struct FailingSink;
impl FrameSink for FailingSink {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
}

/// Counts `split` calls (the pause/resume boundary the wall-clock-stamped ffmpeg sink needs)
/// and records the `ts` each frame carried when it reached the sink - both into handles the
/// test keeps, since the sink itself is boxed away inside the session.
struct SpySink { splits: Arc<AtomicUsize>, seen: Arc<Mutex<Vec<u64>>> }
impl SpySink {
    fn new() -> (Self, Arc<AtomicUsize>, Arc<Mutex<Vec<u64>>>) {
        let (splits, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(Mutex::new(Vec::new())));
        (Self { splits: splits.clone(), seen: seen.clone() }, splits, seen)
    }
}
impl FrameSink for SpySink {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool> {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).push(f.ts.0);
        Ok(true)
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> { Ok(()) }
    fn split(&mut self) -> std::io::Result<()> { self.splits.fetch_add(1, Ordering::SeqCst); Ok(()) }
}

/// Clears `paused` once it has handed out `after` frames, so ONE `run` call really sees a
/// pause -> resume transition (the flag is flipped by the user, mid-loop, in production).
struct ResumingSource { inner: FakeFrameSource, paused: Arc<AtomicBool>, after: usize, seen: usize }
impl crate::capture::frame_source::FrameSource for ResumingSource {
    fn dimensions(&self) -> (u32, u32) { self.inner.dimensions() }
    fn next_frame(&mut self) -> Option<Frame> {
        let f = self.inner.next_frame();
        if f.is_some() {
            self.seen += 1;
            if self.seen == self.after { self.paused.store(false, Ordering::SeqCst); }
        }
        f
    }
    fn drain_latest(&mut self) -> Option<Frame> { self.inner.drain_latest() }
}

#[test]
fn pumps_all_frames_then_reports_exhausted() {
    let mut s = session(vec![frame(0), frame(33), frame(66)], Box::new(FakeFrameSink::default()));
    assert!(s.pump_once()); assert!(s.pump_once()); assert!(s.pump_once());
    assert!(!s.pump_once());
    assert_eq!(s.frames_written(), 3);
}

#[test]
fn run_until_stopped_halts_on_flag() {
    let mut s = session((0..10).map(|i| frame(i * 33)).collect(), Box::new(FakeFrameSink::default()));
    s.run_until_stopped(&AtomicBool::new(true));
    assert_eq!(s.frames_written(), 0);
}

#[test]
fn finalize_returns_count_and_marks_stopped() {
    let mut s = session(vec![frame(0)], Box::new(FakeFrameSink::default()));
    s.pump_once();
    assert_eq!(s.stop_and_finalize().unwrap(), 1);
}

#[test]
fn run_encodes_when_not_paused() {
    let mut s = session(vec![frame(0), frame(33)], Box::new(FakeFrameSink::default()));
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert_eq!(s.frames_written(), 2);
}

#[test]
fn run_discards_frames_while_paused() {
    let mut s = session(vec![frame(0), frame(33)], Box::new(FakeFrameSink::default()));
    s.run(&AtomicBool::new(false), &AtomicBool::new(true));
    assert_eq!(s.frames_written(), 0);
}

#[test]
fn run_shifts_post_pause_timestamps_by_the_ledger_span() {
    // Frame at 1000, then the recorder is paused 1000..3000 with NOT ONE frame arriving in
    // between (the static-desktop case WGC really produces). The next frame, captured at
    // 3100, must still land at 1100: the span comes from the pause/resume toggles, not from
    // whatever the screen happened to be doing.
    //
    // The sink must see that same 1100, not the raw 3100: `VfrSegments` builds its concat
    // offsets out of each part's first RECORDING-clock timestamp, so a raw one there would
    // re-introduce the very span this removes.
    let totals = Arc::new(PauseTotals::new());
    let (sink, _, seen) = SpySink::new();
    let mut s = RecordingSession::new(
        Box::new(FakeFrameSource::new(vec![frame(1000)])), Box::new(sink), totals.clone());
    assert!(s.pump_once());
    totals.pause(1000);
    totals.resume(3000);
    s.source = Box::new(FakeFrameSource::new(vec![frame(3100)]));
    assert!(s.pump_once());
    assert_eq!(s.frame_timestamps(), &[1000, 1100]);
    assert_eq!(*seen.lock().unwrap(), vec![1000, 1100], "sink saw raw capture times, not the recording clock");
}

/// A resume splits the sink exactly once, which is what keeps the paused span out of the
/// wall-clock-stamped ffmpeg output's own PTS - and only frames after the resume are encoded.
#[test]
fn run_splits_the_sink_on_resume() {
    let paused = Arc::new(AtomicBool::new(true));
    let src = ResumingSource {
        inner: FakeFrameSource::new(vec![frame(0), frame(33), frame(66)]),
        paused: paused.clone(), after: 2, seen: 0,
    };
    let (sink, splits, _) = SpySink::new();
    let mut s = RecordingSession::new(Box::new(src), Box::new(sink), Arc::new(PauseTotals::new()));
    s.run(&AtomicBool::new(false), &paused);
    assert_eq!(splits.load(Ordering::SeqCst), 1);
    assert_eq!(s.frames_written(), 1); // only frame(66), after the resume
}

/// A recording that is never paused must never split, so it still writes exactly one file
/// and `finish` costs nothing extra.
#[test]
fn run_never_splits_without_a_pause() {
    let (sink, splits, _) = SpySink::new();
    let mut s = session(vec![frame(0), frame(33)], Box::new(sink));
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert_eq!(splits.load(Ordering::SeqCst), 0);
    assert_eq!(s.frames_written(), 2);
}

/// A frame the sink ERRORED on is not in the video, so it must not be in `sync.json` either -
/// which, since M1's salvage, is written even when the take failed to finalize. Same invariant
/// the GPU path's `record_if_encoded` pins for `send_frame`.
#[test]
fn frames_written_counts_only_successful_pushes() {
    let mut s = session(vec![frame(0), frame(33)], Box::new(FailingSink));
    assert!(s.pump_once());
    assert!(s.pump_once());
    assert!(!s.pump_once());
    assert_eq!(s.frames_written(), 0);
    assert!(s.frame_timestamps().is_empty(), "an unencoded frame reached sync.json");
}
