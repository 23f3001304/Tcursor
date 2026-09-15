use cursor_zoom_lib::capture::frame::Frame;
use cursor_zoom_lib::domain::time::Timestamp;
use cursor_zoom_lib::encode::ffmpeg_encoder::FfmpegFrameSink;
use cursor_zoom_lib::encode::frame_sink::FrameSink;

#[test]
fn encodes_synthetic_frames_to_nonempty_mp4() {
    let dir = std::env::temp_dir();
    let out = dir.join("m1_encode_test.mp4");
    let _ = std::fs::remove_file(&out);
    let (w, h, fps, n) = (320u32, 240u32, 30u32, 10u32);

    let mut sink: Box<dyn FrameSink> =
        Box::new(FfmpegFrameSink::new(out.to_str().unwrap(), w, h, fps).unwrap());
    for i in 0..n {
        let mut bgra = vec![0u8; (w * h * 4) as usize];
        for px in bgra.chunks_mut(4) {
            px[2] = (i * 20) as u8;
            px[3] = 255;
        }
        sink.push(&Frame {
            width: w,
            height: h,
            bgra,
            ts: Timestamp((i * 33) as u64),
        })
        .unwrap();
    }
    sink.finish().unwrap();

    let meta = std::fs::metadata(&out).expect("mp4 should exist");
    assert!(meta.len() > 0, "mp4 should be non-empty");
}
