// Run: cargo test --test manual_system_audio -- --ignored --nocapture
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use cursor_zoom_lib::audio::capture::system_audio::SystemAudio;
use cursor_zoom_lib::domain::time::{Clock, SystemClock};

#[test]
#[ignore]
fn record_two_seconds_of_system_audio() {
    let out = std::env::temp_dir().join("m2_system.wav");
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let started = Arc::new(AtomicU64::new(0));
    // Reading before `loopback` is called brackets the device-open + config-negotiation
    // work (query, WavWriter::create, build_input_stream, play()) that happens before the
    // first callback can fire.
    let open_ms = clock.now_ms();
    let h = SystemAudio::loopback(out.to_str().unwrap(), Arc::new(AtomicBool::new(false)), started.clone(), clock.clone(), None).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    h.stop().unwrap();
    let r = hound::WavReader::open(&out).unwrap();
    let count = r.into_samples::<i16>().count();
    let started_ms = started.load(Ordering::SeqCst);
    println!("captured {count} samples, started={started_ms}ms (pre-open={open_ms}ms)");
    assert!(count > 0);
    // Fix (b): `started` is stamped inside the data callback at the first non-empty packet,
    // which necessarily lands at/after the pre-open reading above - not at stream-open
    // (the old behavior), which would have been closer to `open_ms` regardless of when
    // audio actually started flowing.
    assert!(started_ms >= open_ms, "system audio should stamp at/after open, got {started_ms}ms < {open_ms}ms");
}
