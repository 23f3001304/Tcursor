use super::pause_totals::PauseTotals;
use std::sync::Arc;

const HNS_PER_MS: i64 = 10_000;

pub struct FrameTick {
    pub sync_ms: u64,
    pub pts_100ns: i64,
}

pub struct PauseClock {
    totals: Arc<PauseTotals>,
    base_ms: Option<u64>,
    last_ms: Option<u64>,
}

impl PauseClock {
    pub fn new(totals: Arc<PauseTotals>) -> Self {
        Self {
            totals,
            base_ms: None,
            last_ms: None,
        }
    }

    pub fn tick(&mut self, now_ms: u64, paused: bool) -> Option<FrameTick> {
        if paused {
            return None;
        }
        let sync_ms = self.totals.stamp_ms(now_ms);
        if self.last_ms.is_some_and(|last| sync_ms <= last) {
            return None;
        }
        self.last_ms = Some(sync_ms);
        let base = *self.base_ms.get_or_insert(sync_ms);
        Some(FrameTick {
            sync_ms,
            pts_100ns: (sync_ms - base) as i64 * HNS_PER_MS,
        })
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
        totals.resume(300);
        assert_eq!(pc.tick(400, false).unwrap().sync_ms, 200);
        totals.pause(500);
        totals.resume(650);
        assert_eq!(pc.tick(700, false).unwrap().sync_ms, 350);
    }

    #[test]
    fn tick_during_an_open_pause_is_dropped() {
        let (totals, mut pc) = clock();
        pc.tick(0, false).unwrap();
        totals.pause(100);
        assert!(pc.tick(2000, true).is_none());
    }

    #[test]
    fn encoder_pts_is_exactly_the_sync_timestamp_rebased() {
        let (totals, mut pc) = clock();
        let mut ticks: Vec<FrameTick> = [0u64, 100, 200]
            .iter()
            .map(|&n| pc.tick(n, false).unwrap())
            .collect();
        totals.pause(250);
        totals.resume(30_250);
        ticks.extend(
            [30_300u64, 30_400]
                .iter()
                .map(|&n| pc.tick(n, false).unwrap()),
        );

        assert_eq!(
            ticks.iter().map(|t| t.sync_ms).collect::<Vec<_>>(),
            vec![0, 100, 200, 300, 400]
        );
        let base = ticks[0].sync_ms;
        for t in &ticks {
            assert_eq!(t.pts_100ns, (t.sync_ms - base) as i64 * HNS_PER_MS);
        }
    }

    #[test]
    fn pts_is_relative_to_the_first_encoded_frame() {
        let (totals, mut pc) = clock();
        totals.pause(0);
        totals.resume(5_000);
        assert_eq!(pc.tick(5_100, false).unwrap().pts_100ns, 0);
        assert_eq!(pc.tick(5_200, false).unwrap().pts_100ns, 100 * HNS_PER_MS);
    }

    #[test]
    fn non_advancing_tick_is_dropped() {
        let (totals, mut pc) = clock();
        assert_eq!(pc.tick(1000, false).unwrap().sync_ms, 1000);
        totals.pause(1001);
        totals.resume(1500);
        assert!(pc.tick(1499, false).is_none());
        assert_eq!(pc.tick(1600, false).unwrap().sync_ms, 1101);
    }
}
