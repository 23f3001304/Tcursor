use std::sync::Mutex;

struct PauseState {
    paused_ms: u64,
    pause_started: u64,
}

pub struct PauseTotals {
    state: Mutex<PauseState>,
}

impl PauseTotals {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PauseState {
                paused_ms: 0,
                pause_started: 0,
            }),
        }
    }

    pub fn pause(&self, now_ms: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started != 0 {
            return;
        }
        s.pause_started = now_ms;
    }

    pub fn resume(&self, now_ms: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started == 0 {
            return;
        }
        let started = s.pause_started;
        s.pause_started = 0;
        s.paused_ms += now_ms.saturating_sub(started);
    }

    pub fn elapsed_paused(&self, now_ms: u64) -> u64 {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started == 0 {
            s.paused_ms
        } else {
            s.paused_ms + now_ms.saturating_sub(s.pause_started)
        }
    }

    pub fn stamp_ms(&self, raw_ms: u64) -> u64 {
        raw_ms.saturating_sub(self.elapsed_paused(raw_ms))
    }

    pub fn stamp(&self, raw_ms: u64) -> u32 {
        self.stamp_ms(raw_ms) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_pause_elapsed_is_the_in_progress_span() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        assert_eq!(pt.elapsed_paused(2500), 1500);
    }

    #[test]
    fn closed_episode_elapsed_is_the_full_span() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.resume(3000);
        assert_eq!(pt.elapsed_paused(3100), 2000);
    }

    #[test]
    fn resume_updates_both_fields_as_one_atomic_unit() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.resume(3000);
        assert_eq!(pt.elapsed_paused(3000), 2000);
    }

    #[test]
    fn accumulates_across_multiple_pause_episodes() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.resume(1500);
        pt.pause(2000);
        pt.resume(2300);
        assert_eq!(pt.elapsed_paused(9999), 800);
    }

    #[test]
    fn pause_is_idempotent_when_already_paused() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.pause(1500);
        pt.resume(3000);
        assert_eq!(pt.elapsed_paused(3000), 2000);
    }

    #[test]
    fn resume_is_idempotent_when_not_paused() {
        let pt = PauseTotals::new();
        pt.resume(5000);
        assert_eq!(pt.elapsed_paused(5000), 0);
    }
}
