//! The recorder's own ledger of mid-take source switches (2026-09-14): which extra mic and webcam
//! segments a take produced and which display switches it made, filled by the `switch_mic`,
//! `mark_webcam_segment` and `switch_display` commands while the take runs and written into
//! `sync.json` at Stop (`recorder_threads::save_session_files`). Shared as an `Arc<Mutex<_>>` so a
//! command can push under its own short lock without holding the recorder's.
use std::sync::{Arc, Mutex};
use crate::domain::time::Clock;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::sync::{DisplaySwitch, Segment};

#[derive(Default, Debug)]
pub struct SegmentLog {
    /// `mic_2.wav`, `mic_3.wav`... - never `mic.wav`, the first segment, which `mic_ms` places.
    pub mic: Vec<Segment>,
    /// `webcam_2.webm`... - never `webcam.webm`.
    pub webcam: Vec<Segment>,
    pub displays: Vec<DisplaySwitch>,
}

pub type SharedSegments = Arc<Mutex<SegmentLog>>;

pub fn shared() -> SharedSegments { Arc::new(Mutex::new(SegmentLog::default())) }

impl SegmentLog {
    /// Correct the size of the display switch stamped `at_ms` to what its capture actually
    /// delivered (`gpu_frames::SizeHook`): the render crops the fitted picture by it. An unknown
    /// `at_ms` (the switch failed and its record is gone) changes nothing.
    pub fn set_display_size(&mut self, at_ms: u64, w: u32, h: u32) {
        if let Some(d) = self.displays.iter_mut().rev().find(|d| d.at_ms == at_ms) { d.w = w; d.h = h; }
    }
}

/// Now, on the recording clock: the capture clock with every paused span removed - the same
/// clock `sync.json`'s frame times, `mic_ms` and every event stream are on, so a segment's
/// `start_ms` lands where the merge expects it. Zero until the recording clock has advanced.
pub fn recording_ms(clock: &dyn Clock, totals: &PauseTotals) -> u64 {
    totals.stamp_ms(clock.now_ms())
}

/// The next segment's file name for a source: the first extra segment is `_2`, since the
/// unnumbered file is the first segment.
pub fn next_name(stem: &str, ext: &str, extra_so_far: usize) -> String {
    format!("{stem}_{}.{ext}", extra_so_far + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_segments_count_from_two() {
        assert_eq!(next_name("mic", "wav", 0), "mic_2.wav");
        assert_eq!(next_name("webcam", "webm", 2), "webcam_4.webm");
    }

    /// The switch record starts with the target's window rect and is corrected to the first
    /// frame's size; only the switch stamped at that instant changes, and a stamp with no record
    /// (a failed switch) changes nothing.
    #[test]
    fn a_display_switch_takes_its_capture_s_first_frame_size() {
        let mut log = SegmentLog::default();
        log.displays.push(DisplaySwitch { at_ms: 1000, target_id: "display:1".into(), w: 1280, h: 800 });
        log.displays.push(DisplaySwitch { at_ms: 8091, target_id: "window:0x1".into(), w: 974, h: 1087 });
        log.set_display_size(8091, 960, 1080);
        log.set_display_size(5, 1, 1);
        assert_eq!((log.displays[0].w, log.displays[0].h), (1280, 800));
        assert_eq!((log.displays[1].w, log.displays[1].h), (960, 1080));
    }

    #[test]
    fn the_log_starts_empty_and_is_shared() {
        let a = shared();
        let b = a.clone();
        b.lock().unwrap().mic.push(Segment { path: "mic_2.wav".into(), start_ms: 10 });
        assert_eq!(a.lock().unwrap().mic.len(), 1);
        assert!(a.lock().unwrap().webcam.is_empty());
    }
}
