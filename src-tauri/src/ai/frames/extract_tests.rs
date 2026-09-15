use super::*;

fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn jpeg_dims_reads_the_sof_marker() {
    assert_eq!(jpeg_dims(&[]), None);
    assert_eq!(jpeg_dims(b"not a jpeg at all"), None);
    assert_eq!(
        jpeg_dims(&[0xFF, 0xD8, 0xFF, 0xD9]),
        None,
        "an empty JPEG has no frame header"
    );
}

#[test]
fn extracts_a_frame_at_512_on_the_long_side() {
    if !ffmpeg_present() {
        eprintln!("skipped: no ffmpeg on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!("tcursor-aiframes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let vid = dir.join("probe.mp4");
    let built = crate::process::proc::ffcmd("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=1280x720:rate=30:duration=3",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&vid)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(built, "could not build the probe video");

    let jpg = jpeg_at(&vid, 1_000, LONG_EDGE).expect("a frame at 1s");
    assert!(jpg.starts_with(&[0xFF, 0xD8]), "not a JPEG");
    assert_eq!(
        jpeg_dims(&jpg),
        Some((512, 288)),
        "512 on the long side, aspect kept"
    );
    assert!(
        jpg.len() < 120_000,
        "q80-ish, not a lossless dump: {} bytes",
        jpg.len()
    );

    assert!(
        jpeg_at(&vid, 60_000, LONG_EDGE).is_err(),
        "a seek past the end must be an error, not zero bytes passed off as a frame"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
