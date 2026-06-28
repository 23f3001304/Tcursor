// Run explicitly: cargo test --test manual_capture -- --ignored --nocapture
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use cursor_zoom_lib::capture::frame_source::FrameSource;
use cursor_zoom_lib::capture::windows_capture::WgcFrameSource;
use cursor_zoom_lib::domain::time::{Clock, SystemClock};
use cursor_zoom_lib::encode::ffmpeg_encoder::FfmpegFrameSink;
use cursor_zoom_lib::encode::frame_sink::FrameSink;

#[test]
#[ignore]
fn record_two_seconds_of_primary_display() {
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let mut src = WgcFrameSource::for_primary_display(clock, 30, true).unwrap();
    let (w, h) = src.dimensions();
    let out = std::env::temp_dir().join("m1_capture.mp4");
    let mut sink: Box<dyn FrameSink> =
        Box::new(FfmpegFrameSink::new(out.to_str().unwrap(), w, h, 30).unwrap());

    let stop = AtomicBool::new(false);
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 2 {
        if let Some(f) = src.next_frame() {
            sink.push(&f).unwrap();
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
    }
    sink.finish().unwrap();
    println!("wrote {}", out.display());
    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}
