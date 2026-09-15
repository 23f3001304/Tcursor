use cursor_zoom_lib::audio::capture::system_audio::SystemAudio;
use cursor_zoom_lib::domain::time::{Clock, SystemClock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[test]
#[ignore]
fn record_two_seconds_of_system_audio() {
    let out = std::env::temp_dir().join("m2_system.wav");
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let started = Arc::new(AtomicU64::new(0));
    let open_ms = clock.now_ms();
    let platform = cursor_zoom_lib::platform::current();
    let h = SystemAudio::loopback(
        platform.audio.loopback_device(),
        out.to_str().unwrap(),
        Arc::new(AtomicBool::new(false)),
        started.clone(),
        clock.clone(),
        None,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    h.stop().unwrap();
    let r = hound::WavReader::open(&out).unwrap();
    let count = r.into_samples::<i16>().count();
    let started_ms = started.load(Ordering::SeqCst);
    println!("captured {count} samples, started={started_ms}ms (pre-open={open_ms}ms)");
    assert!(count > 0);
    assert!(
        started_ms >= open_ms,
        "system audio should stamp at/after open, got {started_ms}ms < {open_ms}ms"
    );
}
