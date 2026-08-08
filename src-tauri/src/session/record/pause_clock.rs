//! Shared paused-time accumulator for both capture paths (`gpu_record::Cap` and
//! `recording_session::RecordingSession`). VFR capture only ticks on frame arrival —
//! there is no dedicated "check for resume" moment — so the accumulator works purely
//! from a stream of per-frame `(now, paused)` samples rather than a tight poll loop
//! (contrast `session::pacing::run_paced`, which owns its own 2ms poll loop and can
//! read the clock at the exact instant it notices a resume).

/// Accumulates paused wall-clock time from a stream of per-frame `(now, paused)`
/// samples and shifts capture timestamps to exclude it, so a pause leaves no gap
/// between `sync.json` and `video.mp4`.
#[derive(Default)]
pub struct PauseClock {
    paused_ms: u64,
    last_paused_at: Option<u64>,
}

impl PauseClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one frame-arrival tick at time `now` (ms). While `paused`, returns
    /// `None` (the frame should be dropped) and accumulates elapsed time against the
    /// *previous* paused sample, if any — consecutive paused ticks measure the real
    /// paused span even though no sample lands exactly on the pause/resume instant.
    /// A lone paused sample (no predecessor yet) contributes no duration. When not
    /// paused, returns `Some(now - paused_ms)`, the timestamp to record.
    pub fn observe(&mut self, now: u64, paused: bool) -> Option<u64> {
        if paused {
            if let Some(prev) = self.last_paused_at {
                self.paused_ms += now.saturating_sub(prev);
            }
            self.last_paused_at = Some(now);
            return None;
        }
        self.last_paused_at = None;
        Some(now.saturating_sub(self.paused_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpaused_tick_before_any_pause_is_unadjusted() {
        assert_eq!(PauseClock::new().observe(1000, false), Some(1000));
    }

    #[test]
    fn paused_ticks_return_none_and_drop_the_frame() {
        let mut pc = PauseClock::new();
        assert_eq!(pc.observe(1000, true), None);
        assert_eq!(pc.observe(1500, true), None);
    }

    #[test]
    fn push_after_pause_excludes_the_paused_span() {
        // push@1000, paused 1000..3000, push@3100 -> 1100: the 2000ms pause
        // (measured between the two paused samples) is subtracted from every
        // timestamp recorded afterward. This is the brief's required scenario.
        let mut pc = PauseClock::new();
        assert_eq!(pc.observe(1000, false), Some(1000));
        assert_eq!(pc.observe(1000, true), None);
        assert_eq!(pc.observe(3000, true), None);
        assert_eq!(pc.observe(3100, false), Some(1100));
    }

    #[test]
    fn accumulates_across_multiple_pause_episodes() {
        let mut pc = PauseClock::new();
        assert_eq!(pc.observe(0, false), Some(0));
        assert_eq!(pc.observe(100, true), None);
        assert_eq!(pc.observe(300, true), None); // pause #1: 200ms
        assert_eq!(pc.observe(400, false), Some(200));
        assert_eq!(pc.observe(500, true), None);
        assert_eq!(pc.observe(650, true), None); // pause #2: 150ms
        assert_eq!(pc.observe(700, false), Some(350)); // 700 - (200 + 150)
    }

    #[test]
    fn single_paused_sample_contributes_no_duration_yet() {
        let mut pc = PauseClock::new();
        assert_eq!(pc.observe(1000, false), Some(1000));
        assert_eq!(pc.observe(2000, true), None); // no prior paused sample to diff
        assert_eq!(pc.observe(2001, false), Some(2001)); // paused_ms still 0
    }
}
