// Split from windows_capture.rs per repo convention (#[path] sibling test module).
// Exercises `try_send_or_drop` directly: the surrounding WGC callback needs a live
// capture session and isn't practical to unit test, but the backpressure decision it
// delegates to is a plain function over a real `SyncSender`/`AtomicU64`.
use super::*;
use std::sync::mpsc::sync_channel;
use crate::domain::time::Timestamp;

fn frame() -> Frame {
    Frame { width: 2, height: 2, bgra: vec![0; 2 * 2 * 4], ts: Timestamp(0) }
}

#[test]
fn sends_and_leaves_drop_count_at_zero_when_queue_has_room() {
    let (tx, rx) = sync_channel(1);
    let drops = AtomicU64::new(0);
    try_send_or_drop(&tx, frame(), &drops);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert!(rx.try_recv().is_ok());
}

#[test]
fn drops_and_counts_instead_of_blocking_when_queue_is_full() {
    let (tx, _rx) = sync_channel(1); // never drained, so slot 2 always finds it full
    let drops = AtomicU64::new(0);
    try_send_or_drop(&tx, frame(), &drops); // fills the one slot
    try_send_or_drop(&tx, frame(), &drops); // full -> dropped, not blocked
    try_send_or_drop(&tx, frame(), &drops); // still full -> dropped again
    assert_eq!(drops.load(Ordering::Relaxed), 2);
}
