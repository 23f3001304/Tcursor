// Exact-span paused-time ledger for the input trackers (mouse, keyboard actions/typing,
// cursor-type). Stamped directly by the SAME code path that flips the recorder's `paused`
// AtomicBool (`pause_recording` / `resume_recording`), which knows the exact wall-clock
// instant the toggle happens - unlike `PauseClock` (tick-based, inferred from a stream of
// frame-arrival samples; that model stays correct for the video path and is untouched here).
//
// `paused_ms` and `pause_started` are one `Mutex`-guarded unit, not two independent atomics:
// `resume` must clear `pause_started` and fold its span into `paused_ms` as a single visible
// step, or a tracker thread reading `elapsed_paused` between those two writes (a hook/poll
// thread is never serialized against `resume_recording` - clicking Resume is itself a mouse
// event the hook observes at that same instant) would see "not paused" paired with the
// PRE-update `paused_ms`, under-counting by up to the whole just-closed span. A `Mutex` makes
// that torn read structurally impossible rather than relying on write ordering (which only
// moves the race to a different pair of fields) or a timing-dependent test.

use std::sync::Mutex;

struct PauseState {
    paused_ms: u64,
    pause_started: u64, // 0 = not currently paused
}

pub struct PauseTotals {
    state: Mutex<PauseState>,
}

impl PauseTotals {
    pub fn new() -> Self {
        Self { state: Mutex::new(PauseState { paused_ms: 0, pause_started: 0 }) }
    }

    // Record a pause starting at `now_ms`. Idempotent: a second call while already paused
    // is a no-op, keeping the original anchor so the span isn't shortened.
    pub fn pause(&self, now_ms: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started != 0 { return; }
        s.pause_started = now_ms;
    }

    // Close the in-progress pause as of `now_ms`, folding its span into `paused_ms`.
    // Idempotent: a call while not paused is a no-op (no double-count). Clearing
    // `pause_started` and updating `paused_ms` happen under one lock acquisition, so no
    // other thread can observe the half-updated state in between.
    pub fn resume(&self, now_ms: u64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started == 0 { return; }
        let started = s.pause_started;
        s.pause_started = 0;
        s.paused_ms += now_ms.saturating_sub(started);
    }

    // Accumulated + in-progress paused span as of `now_ms`, read as one consistent pair.
    pub fn elapsed_paused(&self, now_ms: u64) -> u64 {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.pause_started == 0 { s.paused_ms } else { s.paused_ms + now_ms.saturating_sub(s.pause_started) }
    }

    // Pause-adjust a tracker's raw elapsed-ms reading: subtract `elapsed_paused` at that
    // same instant. Every input tracker calls this at its stamp site instead of the raw value.
    pub fn stamp(&self, raw_ms: u64) -> u32 {
        raw_ms.saturating_sub(self.elapsed_paused(raw_ms)) as u32
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

    // Pins the invariant the two-atomic layout could tear on: `pause_started` clearing and
    // `paused_ms` updating are a single Mutex-guarded write, so any `elapsed_paused` call
    // (this test only exercises it single-threaded, but the guarantee is structural - the
    // lock makes an interleaved read of "cleared pause_started, stale paused_ms" impossible,
    // not just unlikely) always sees the fully-updated pair together.
    #[test]
    fn resume_updates_both_fields_as_one_atomic_unit() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.resume(3000);
        assert_eq!(pt.elapsed_paused(3000), 2000); // t1 - t0, observed via the combined state
    }

    #[test]
    fn accumulates_across_multiple_pause_episodes() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.resume(1500); // +500
        pt.pause(2000);
        pt.resume(2300); // +300
        assert_eq!(pt.elapsed_paused(9999), 800);
    }

    #[test]
    fn pause_is_idempotent_when_already_paused() {
        let pt = PauseTotals::new();
        pt.pause(1000);
        pt.pause(1500); // no-op: must not move the anchor forward
        pt.resume(3000);
        assert_eq!(pt.elapsed_paused(3000), 2000); // not 1500
    }

    #[test]
    fn resume_is_idempotent_when_not_paused() {
        let pt = PauseTotals::new();
        pt.resume(5000); // no-op: never paused
        assert_eq!(pt.elapsed_paused(5000), 0);
    }
}
