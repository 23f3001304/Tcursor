// Run: cargo test --test manual_system_audio -- --ignored --nocapture
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use cursor_zoom_lib::audio::capture::system_audio::SystemAudio;

#[test]
#[ignore]
fn record_two_seconds_of_system_audio() {
    let out = std::env::temp_dir().join("m2_system.wav");
    let h = SystemAudio::loopback(out.to_str().unwrap(), Arc::new(AtomicBool::new(false))).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    h.stop().unwrap();
    let r = hound::WavReader::open(&out).unwrap();
    let count = r.into_samples::<i16>().count();
    println!("captured {} samples", count);
    assert!(count > 0);
}
