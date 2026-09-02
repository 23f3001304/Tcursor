use super::*;

// `Running` holds real thread handles/trackers that need a live Tauri app to construct, so these
// exercise `is_recording`/`is_busy` purely through the two fields they actually read - exactly
// what `stop_blocking` manipulates (`inner.take()` then `stopping`), without needing a full
// recording session.

#[test]
fn idle_recorder_is_neither_recording_nor_busy() {
    let r = Recorder::default();
    assert!(!r.is_recording());
    assert!(!r.is_busy());
}

#[test]
fn stopping_alone_is_busy_but_not_recording() {
    // Mirrors the window `stop_blocking` opens: `inner` already taken (`None`), `stopping` still
    // true because the finalize hasn't returned yet - the exact gap `is_busy` exists to cover for
    // the `CloseRequested` guard.
    let r = Recorder::default();
    r.stopping.store(true, Ordering::SeqCst);
    assert!(!r.is_recording());
    assert!(r.is_busy());
}
