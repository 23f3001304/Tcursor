use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub const ZERO: Timestamp = Timestamp(0);
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

impl std::ops::Sub for Timestamp {
    type Output = u64;
    fn sub(self, rhs: Timestamp) -> u64 {
        self.0 - rhs.0
    }
}

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock {
    start: Instant,
}
impl SystemClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
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
}
