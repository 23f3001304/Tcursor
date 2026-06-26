use cursor_zoom_lib::audio::cpal_mic::CpalMic;

#[test]
#[ignore]
fn record_two_seconds_of_mic() {
    let out = std::env::temp_dir().join("m1_mic.wav");
    let handle = CpalMic::default_input(out.to_str().unwrap()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    handle.stop().unwrap();
    println!("wrote {}", out.display());
    let r = hound::WavReader::open(&out).unwrap();
    assert!(r.into_samples::<i16>().count() > 0);
}
