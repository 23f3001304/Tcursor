//! A tiny recycled-buffer pool over an mpsc channel, so the export pipeline reuses frame
//! buffers between decode -> composite -> encode instead of allocating ~tens of MB per frame.
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct BufPool {
    free: Receiver<Vec<u8>>,
    ret: Sender<Vec<u8>>,
    bytes: usize,
}

impl BufPool {
    /// Pre-seed `count` zeroed buffers of `bytes` each.
    pub fn new(count: usize, bytes: usize) -> Self {
        let (ret, free) = channel();
        for _ in 0..count { let _ = ret.send(vec![0u8; bytes]); }
        Self { free, ret, bytes }
    }
    /// A free buffer; allocates a fresh one rather than deadlock if none is
    /// available (e.g. a consumer transiently holds them all). Steady state
    /// hits the recycled path; the bounded pipeline channel provides backpressure.
    pub fn take(&self) -> Vec<u8> {
        match self.free.try_recv() {
            Ok(b) => b,
            Err(_) => vec![0u8; self.bytes],
        }
    }
    /// Return a buffer for reuse (dropped if the pool is gone).
    pub fn put(&self, buf: Vec<u8>) { let _ = self.ret.send(buf); }
    /// A clonable handle to the return side, for the encoder thread.
    pub fn returner(&self) -> Sender<Vec<u8>> { self.ret.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn take_returns_seeded_buffers_of_the_right_size() {
        let p = BufPool::new(2, 16);
        let a = p.take(); assert_eq!(a.len(), 16);
        let b = p.take(); assert_eq!(b.len(), 16);
    }
    #[test]
    fn put_then_take_recycles_the_same_allocation() {
        let p = BufPool::new(1, 8);
        let mut a = p.take(); a[0] = 7;
        let ptr = a.as_ptr();
        p.put(a);
        let b = p.take();
        assert_eq!(b.as_ptr(), ptr, "recycled the same Vec, no realloc");
    }
    #[test]
    fn take_never_deadlocks_when_pool_exhausted_and_returns_pending() {
        // With count=1 and the only buffer still out, take() must allocate rather than hang.
        let p = BufPool::new(1, 4);
        let _held = p.take();
        let extra = p.take(); // must not block forever
        assert_eq!(extra.len(), 4);
    }
}
