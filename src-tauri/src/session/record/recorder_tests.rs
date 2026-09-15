use super::*;

#[test]
fn idle_recorder_is_neither_recording_nor_busy() {
    let r = Recorder::default();
    assert!(!r.is_recording());
    assert!(!r.is_busy());
}

#[test]
fn stopping_alone_is_busy_but_not_recording() {
    let r = Recorder::default();
    r.stopping.store(true, Ordering::SeqCst);
    assert!(!r.is_recording());
    assert!(r.is_busy());
}
