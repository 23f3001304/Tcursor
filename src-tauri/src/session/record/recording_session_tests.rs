use super::*;
use crate::capture::frame::Frame;
use crate::capture::frame_source::FakeFrameSource;
use crate::domain::time::Timestamp;
use crate::encode::frame_sink::FakeFrameSink;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

fn frame(ts: u64) -> Frame {
    Frame {
        width: 2,
        height: 2,
        bgra: vec![0; 2 * 2 * 4],
        ts: Timestamp(ts),
    }
}

fn session(frames: Vec<Frame>, sink: Box<dyn FrameSink>) -> RecordingSession {
    RecordingSession::new(
        Box::new(FakeFrameSource::new(frames)),
        sink,
        Arc::new(PauseTotals::new()),
    )
}

struct FailingSink;
impl FrameSink for FailingSink {
    fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "fail"))
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> {
        Ok(())
    }
}

struct SpySink {
    splits: Arc<AtomicUsize>,
    seen: Arc<Mutex<Vec<u64>>>,
}
impl SpySink {
    fn new() -> (Self, Arc<AtomicUsize>, Arc<Mutex<Vec<u64>>>) {
        let (splits, seen) = (
            Arc::new(AtomicUsize::new(0)),
            Arc::new(Mutex::new(Vec::new())),
        );
        (
            Self {
                splits: splits.clone(),
                seen: seen.clone(),
            },
            splits,
            seen,
        )
    }
}
impl FrameSink for SpySink {
    fn push(&mut self, f: &Frame) -> std::io::Result<bool> {
        self.seen
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(f.ts.0);
        Ok(true)
    }
    fn finish(self: Box<Self>) -> std::io::Result<()> {
        Ok(())
    }
    fn split(&mut self) -> std::io::Result<()> {
        self.splits.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct ResumingSource {
    inner: FakeFrameSource,
    paused: Arc<AtomicBool>,
    after: usize,
    seen: usize,
}
impl crate::capture::frame_source::FrameSource for ResumingSource {
    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }
    fn next_frame(&mut self) -> Option<Frame> {
        let f = self.inner.next_frame();
        if f.is_some() {
            self.seen += 1;
            if self.seen == self.after {
                self.paused.store(false, Ordering::SeqCst);
            }
        }
        f
    }
    fn drain_latest(&mut self) -> Option<Frame> {
        self.inner.drain_latest()
    }
}

#[test]
fn pumps_all_frames_then_reports_exhausted() {
    let mut s = session(
        vec![frame(0), frame(33), frame(66)],
        Box::new(FakeFrameSink::default()),
    );
    assert!(s.pump_once());
    assert!(s.pump_once());
    assert!(s.pump_once());
    assert!(!s.pump_once());
    assert_eq!(s.frames_written(), 3);
}

#[test]
fn run_until_stopped_halts_on_flag() {
    let mut s = session(
        (0..10).map(|i| frame(i * 33)).collect(),
        Box::new(FakeFrameSink::default()),
    );
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
    let mut s = session(
        vec![frame(0), frame(33)],
        Box::new(FakeFrameSink::default()),
    );
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert_eq!(s.frames_written(), 2);
}

#[test]
fn run_discards_frames_while_paused() {
    let mut s = session(
        vec![frame(0), frame(33)],
        Box::new(FakeFrameSink::default()),
    );
    s.run(&AtomicBool::new(false), &AtomicBool::new(true));
    assert_eq!(s.frames_written(), 0);
}

#[test]
fn run_shifts_post_pause_timestamps_by_the_ledger_span() {
    let totals = Arc::new(PauseTotals::new());
    let (sink, _, seen) = SpySink::new();
    let mut s = RecordingSession::new(
        Box::new(FakeFrameSource::new(vec![frame(1000)])),
        Box::new(sink),
        totals.clone(),
    );
    assert!(s.pump_once());
    totals.pause(1000);
    totals.resume(3000);
    s.source = Box::new(FakeFrameSource::new(vec![frame(3100)]));
    assert!(s.pump_once());
    assert_eq!(s.frame_timestamps(), &[1000, 1100]);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![1000, 1100],
        "sink saw raw capture times, not the recording clock"
    );
}

#[test]
fn run_splits_the_sink_on_resume() {
    let paused = Arc::new(AtomicBool::new(true));
    let src = ResumingSource {
        inner: FakeFrameSource::new(vec![frame(0), frame(33), frame(66)]),
        paused: paused.clone(),
        after: 2,
        seen: 0,
    };
    let (sink, splits, _) = SpySink::new();
    let mut s = RecordingSession::new(Box::new(src), Box::new(sink), Arc::new(PauseTotals::new()));
    s.run(&AtomicBool::new(false), &paused);
    assert_eq!(splits.load(Ordering::SeqCst), 1);
    assert_eq!(s.frames_written(), 1);
}

#[test]
fn run_never_splits_without_a_pause() {
    let (sink, splits, _) = SpySink::new();
    let mut s = session(vec![frame(0), frame(33)], Box::new(sink));
    s.run(&AtomicBool::new(false), &AtomicBool::new(false));
    assert_eq!(splits.load(Ordering::SeqCst), 0);
    assert_eq!(s.frames_written(), 2);
}

#[test]
fn frames_written_counts_only_successful_pushes() {
    let mut s = session(vec![frame(0), frame(33)], Box::new(FailingSink));
    assert!(s.pump_once());
    assert!(s.pump_once());
    assert!(!s.pump_once());
    assert_eq!(s.frames_written(), 0);
    assert!(
        s.frame_timestamps().is_empty(),
        "an unencoded frame reached sync.json"
    );
}
