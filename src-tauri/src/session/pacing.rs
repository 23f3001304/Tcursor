use crate::capture::frame::Frame;
use crate::capture::frame_source::FrameSource;
use crate::domain::time::{Clock, Timestamp};
use crate::encode::frame_sink::FrameSink;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn frames_due(active: u64, fps: u32) -> u64 {
    active * fps as u64 / 1000 + 1
}

#[allow(clippy::too_many_arguments)]
pub fn emit_due(
    source: &mut dyn FrameSource,
    sink: &mut dyn FrameSink,
    frames: &mut u64,
    frame_ts: &mut Vec<u64>,
    start: u64,
    active: u64,
    fps: u32,
    emitted: u64,
    latest: &mut Frame,
) -> u64 {
    let due = frames_due(active, fps);
    let mut k = emitted;
    while k < due {
        if let Some(f) = source.drain_latest() {
            *latest = f;
        }
        let ts = start + k * 1000 / fps as u64;
        let framed = Frame {
            width: latest.width,
            height: latest.height,
            bgra: latest.bgra.clone(),
            ts: Timestamp(ts),
        };
        match sink.push(&framed) {
            Ok(true) => {
                *frames += 1;
                frame_ts.push(ts);
            }
            Ok(false) => {}
            Err(e) => eprintln!("frame sink push failed: {e}"),
        }
        k += 1;
    }
    k
}

#[allow(clippy::too_many_arguments)]
pub fn run_paced(
    source: &mut dyn FrameSource,
    sink: &mut dyn FrameSink,
    frames: &mut u64,
    frame_ts: &mut Vec<u64>,
    stop: &AtomicBool,
    paused: &AtomicBool,
    clock: &dyn Clock,
    fps: u32,
) {
    let mut latest = loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        if let Some(f) = source.drain_latest() {
            break f;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let start = clock.now_ms();
    let mut emitted = 0u64;
    let mut paused_ms = 0u64;
    let mut pause_at: Option<u64> = None;
    while !stop.load(Ordering::SeqCst) {
        if paused.load(Ordering::SeqCst) {
            if pause_at.is_none() {
                pause_at = Some(clock.now_ms());
            }
            if let Some(f) = source.drain_latest() {
                latest = f;
            }
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        if let Some(t0) = pause_at.take() {
            paused_ms += clock.now_ms().saturating_sub(t0);
        }
        let active = clock
            .now_ms()
            .saturating_sub(start)
            .saturating_sub(paused_ms);
        emitted = emit_due(
            source,
            sink,
            frames,
            frame_ts,
            start,
            active,
            fps,
            emitted,
            &mut latest,
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::frame_source::FakeFrameSource;
    use crate::encode::frame_sink::FakeFrameSink;

    fn frame(ts: u64) -> Frame {
        Frame {
            width: 2,
            height: 2,
            bgra: vec![0; 16],
            ts: Timestamp(ts),
        }
    }

    #[test]
    fn frames_due_counts_from_first_and_excludes_paused() {
        assert_eq!(frames_due(0, 60), 1);
        assert_eq!(frames_due(50, 60), 4);
        assert_eq!(frames_due(1000, 60), 61);
    }

    #[test]
    fn emit_due_emits_uniform_timestamps_and_duplicates() {
        let mut src = FakeFrameSource::new(vec![frame(999)]);
        let mut sink = FakeFrameSink::default();
        let (mut frames, mut ts) = (0u64, Vec::new());
        let mut latest = frame(999);
        let n = emit_due(
            &mut src,
            &mut sink,
            &mut frames,
            &mut ts,
            1000,
            0,
            60,
            0,
            &mut latest,
        );
        assert_eq!(n, 1);
        let n = emit_due(
            &mut src,
            &mut sink,
            &mut frames,
            &mut ts,
            1000,
            50,
            60,
            n,
            &mut latest,
        );
        assert_eq!(n, 4);
        assert_eq!(ts, vec![1000, 1016, 1033, 1050]);
        assert_eq!(frames, 4);
    }

    struct SkippingSink;
    impl crate::encode::frame_sink::FrameSink for SkippingSink {
        fn push(&mut self, _f: &Frame) -> std::io::Result<bool> {
            Ok(false)
        }
        fn finish(self: Box<Self>) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn emit_due_does_not_count_or_timestamp_skipped_frames() {
        let mut src = FakeFrameSource::new(vec![frame(999)]);
        let mut sink = SkippingSink;
        let (mut frames, mut ts) = (0u64, Vec::new());
        let mut latest = frame(999);
        let n = emit_due(
            &mut src,
            &mut sink,
            &mut frames,
            &mut ts,
            1000,
            50,
            60,
            0,
            &mut latest,
        );
        assert_eq!(n, 4);
        assert_eq!(frames, 0);
        assert!(ts.is_empty());
    }
}
