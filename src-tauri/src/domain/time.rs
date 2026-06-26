use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub const ZERO: Timestamp = Timestamp(0);
    pub fn as_millis(&self) -> u64 { self.0 }
}

impl std::ops::Sub for Timestamp {
    type Output = u64;
    fn sub(self, rhs: Timestamp) -> u64 { self.0 - rhs.0 }
}

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Wall-clock relative to creation, in milliseconds.
pub struct SystemClock { start: Instant }
impl SystemClock { pub fn new() -> Self { Self { start: Instant::now() } } }
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 { self.start.elapsed().as_millis() as u64 }
}

/// Deterministic clock for tests.
pub struct FakeClock { ms: AtomicU64 }
impl FakeClock {
    pub fn new(start: u64) -> Self { Self { ms: AtomicU64::new(start) } }
    pub fn advance(&self, ms: u64) { self.ms.fetch_add(ms, Ordering::SeqCst); }
}
impl Clock for FakeClock {
    fn now_ms(&self) -> u64 { self.ms.load(Ordering::SeqCst) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_subtracts_to_elapsed_millis() {
        let a = Timestamp(1000);
        let b = Timestamp(1750);
        assert_eq!(b - a, 750);
    }

    #[test]
    fn fake_clock_advances() {
        let clock = FakeClock::new(500);
        assert_eq!(clock.now_ms(), 500);
        clock.advance(250);
        assert_eq!(clock.now_ms(), 750);
    }
}
