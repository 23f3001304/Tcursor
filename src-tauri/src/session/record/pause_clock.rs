//! The video paths' view of the exact-span paused-time ledger (`PauseTotals`), which is stamped
//! at the real pause/resume instants under the recorder lock. Both capture paths ask it, once per
//! arriving frame, where that frame sits on the ONE recording clock: the `sync.json` timestamp
//! and the encoder PTS come out of a SINGLE ledger read, so `video.mp4`'s own timeline and
//! `sync.json` - and therefore the audio WAVs, cursor, actions and zoom streams, which subtract
//! the same ledger - cannot disagree about how long a pause was.
//!
//! This used to infer the paused span from the gap between consecutive PAUSED frame arrivals.
//! WGC delivers frames on content change (`MinimumUpdateIntervalSettings` is a floor on the
//! rate, not a heartbeat), so a pause over a static desktop produced at most one paused sample
//! and removed 0% of the pause, while a pause over a busy desktop removed ~100% of it: the
//! recording's reported duration was a function of what the screen happened to be doing.

use std::sync::Arc;
use super::pause_totals::PauseTotals;

/// 100-nanosecond units per millisecond - the Media Foundation encoder's timestamp unit.
const HNS_PER_MS: i64 = 10_000;

/// Where one arriving frame sits on the recording clock.
pub struct FrameTick {
    /// Capture time with every paused span removed - the value written to `sync.json`.
    pub sync_ms: u64,
    /// That same instant as an encoder PTS: 100ns units, zero at the first encoded frame.
    pub pts_100ns: i64,
}

pub struct PauseClock {
    totals: Arc<PauseTotals>,
    base_ms: Option<u64>,
    last_ms: Option<u64>,
}

impl PauseClock {
    pub fn new(totals: Arc<PauseTotals>) -> Self {
        Self { totals, base_ms: None, last_ms: None }
    }

    /// Place the frame that arrived at wall-clock `now_ms`. `None` means "drop this frame":
    /// either capture is `paused`, or the pause-compressed time did not advance past the last
    /// encoded frame. The second case is reachable when a whole pause/resume lands between
    /// reading the clock and reading the pause flag; encoding it would hand the encoder a
    /// non-increasing PTS and put a duplicate timestamp in `sync.json`.
    pub fn tick(&mut self, now_ms: u64, paused: bool) -> Option<FrameTick> {
        if paused { return None; }
        let sync_ms = self.totals.stamp_ms(now_ms);
        if self.last_ms.is_some_and(|last| sync_ms <= last) { return None; }
        self.last_ms = Some(sync_ms);
        let base = *self.base_ms.get_or_insert(sync_ms);
        Some(FrameTick { sync_ms, pts_100ns: (sync_ms - base) as i64 * HNS_PER_MS })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> (Arc<PauseTotals>, PauseClock) {
        let totals = Arc::new(PauseTotals::new());
        (totals.clone(), PauseClock::new(totals))
    }

    #[test]
    fn first_tick_is_unadjusted_and_starts_the_pts_at_zero() {
        let (_t, mut pc) = clock();
        let t = pc.tick(1000, false).unwrap();
        assert_eq!(t.sync_ms, 1000);
        assert_eq!(t.pts_100ns, 0);
    }

    #[test]
    fn paused_ticks_are_dropped() {
        let (_t, mut pc) = clock();
        assert!(pc.tick(1000, true).is_none());
        assert!(pc.tick(1500, true).is_none());
    }

    /// H2: a pause is measured from the pause/resume toggles, so a pause over a completely
    /// static screen - where NO frame arrives between them, the case the old frame-delta
    /// accumulator scored as 0ms - is still removed in full.
    #[test]
    fn pause_with_no_frames_at_all_is_still_fully_removed() {
        let (totals, mut pc) = clock();
        assert_eq!(pc.tick(1000, false).unwrap().sync_ms, 1000);
        totals.pause(1000);
        totals.resume(3000);
        assert_eq!(pc.tick(3100, false).unwrap().sync_ms, 1100);
    }

    #[test]
    fn accumulates_across_multiple_pause_episodes() {
        let (totals, mut pc) = clock();
        assert_eq!(pc.tick(0, false).unwrap().sync_ms, 0);
        totals.pause(100);
        totals.resume(300); // +200
        assert_eq!(pc.tick(400, false).unwrap().sync_ms, 200);
        totals.pause(500);
        totals.resume(650); // +150
        assert_eq!(pc.tick(700, false).unwrap().sync_ms, 350);
    }

    /// A tick taken mid-pause (the ledger's in-progress span counts) still reports the frame
    /// as droppable, so the encoder never sees the paused span at all.
    #[test]
    fn tick_during_an_open_pause_is_dropped() {
        let (totals, mut pc) = clock();
        pc.tick(0, false).unwrap();
        totals.pause(100);
        assert!(pc.tick(2000, true).is_none());
    }

    /// C1: the number handed to the encoder as this frame's PTS IS the number written to
    /// `sync.json`, measured from the first encoded frame - one clock, by construction, across
    /// a 30s pause that no frame arrival ever observed. Before this, `video.mp4` kept the
    /// paused span in its own PTS while sync.json/audio/cursor dropped it.
    #[test]
    fn encoder_pts_is_exactly_the_sync_timestamp_rebased() {
        let (totals, mut pc) = clock();
        let mut ticks: Vec<FrameTick> = [0u64, 100, 200].iter().map(|&n| pc.tick(n, false).unwrap()).collect();
        totals.pause(250);
        totals.resume(30_250); // 30s pause, zero frames delivered in between
        ticks.extend([30_300u64, 30_400].iter().map(|&n| pc.tick(n, false).unwrap()));

        assert_eq!(ticks.iter().map(|t| t.sync_ms).collect::<Vec<_>>(), vec![0, 100, 200, 300, 400]);
        let base = ticks[0].sync_ms;
        for t in &ticks {
            assert_eq!(t.pts_100ns, (t.sync_ms - base) as i64 * HNS_PER_MS);
        }
    }

    /// The encoder PTS is relative to the first ENCODED frame, not to clock zero, so a
    /// recording whose first frames arrive late (or during a leading pause) still starts at 0.
    #[test]
    fn pts_is_relative_to_the_first_encoded_frame() {
        let (totals, mut pc) = clock();
        totals.pause(0);
        totals.resume(5_000); // paused before a single frame ever arrived
        assert_eq!(pc.tick(5_100, false).unwrap().pts_100ns, 0);
        assert_eq!(pc.tick(5_200, false).unwrap().pts_100ns, 100 * HNS_PER_MS);
    }

    /// A pause/resume that lands between the clock read and the pause-flag read can compress
    /// two arrivals onto the same instant; the second is dropped rather than given a
    /// non-increasing PTS (and a duplicate `sync.json` entry).
    #[test]
    fn non_advancing_tick_is_dropped() {
        let (totals, mut pc) = clock();
        assert_eq!(pc.tick(1000, false).unwrap().sync_ms, 1000);
        totals.pause(1001);
        totals.resume(1500); // 499ms
        assert!(pc.tick(1499, false).is_none()); // 1499 - 499 = 1000, no advance
        assert_eq!(pc.tick(1600, false).unwrap().sync_ms, 1101);
    }
}
