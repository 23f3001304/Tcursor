use super::*;

fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn the_stream_loops_forever_is_muted_and_covers_the_output() {
    let a = bg_decode_args(Path::new("loop.mp4"), 1920, 1080, 60);
    let i = a.iter().position(|x| x == "-i").unwrap();
    let sl = a
        .iter()
        .position(|x| x == "-stream_loop")
        .expect("no -stream_loop");
    assert!(
        sl < i,
        "-stream_loop is an INPUT option and must precede -i: {a:?}"
    );
    assert_eq!(a[sl + 1], "-1");
    assert!(
        a.iter().any(|x| x == "-an"),
        "a background never carries audio: {a:?}"
    );
    let r = a.iter().rposition(|x| x == "-r").unwrap();
    assert!(
        r > i && a[r + 1] == "60.0000",
        "the output rate IS the output clock: {a:?}"
    );
    let vf = a.iter().position(|x| x == "-vf").unwrap();
    assert_eq!(
        a[vf + 1],
        "scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080"
    );
    let pf = a.iter().position(|x| x == "-pix_fmt").unwrap();
    assert_eq!(
        a[pf + 1],
        "bgra",
        "the bg buffer the compositor uploads is BGRA"
    );
}

#[test]
fn a_two_frame_clip_loops_so_frame_three_is_frame_one() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let clip = std::env::temp_dir().join(format!("tcursor_bgloop_{}.mp4", std::process::id()));
    if !encode_two_colour_clip(&clip) {
        eprintln!("SKIPPED: this ffmpeg could not build the fixture");
        return;
    }
    let mut p = BgPipe::open(&clip, (4, 4), 0.0, 2, 1).expect("open");
    let f: Vec<Vec<u8>> = (0..3).map(|_| p.take().expect("frame")).collect();
    assert_eq!(
        f[0].len(),
        4 * 4 * 4,
        "cover-scaled to the output size by ffmpeg, not by us"
    );
    assert_ne!(
        f[0], f[1],
        "the fixture's two frames must differ, else this proves nothing"
    );
    assert_eq!(
        f[0], f[2],
        "-stream_loop -1 must wrap frame 3 back onto frame 1"
    );
    let _ = std::fs::remove_file(&clip);
}

#[test]
fn dim_is_applied_to_every_streamed_frame() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let clip = std::env::temp_dir().join(format!("tcursor_bgdim_{}.mp4", std::process::id()));
    if !encode_two_colour_clip(&clip) {
        eprintln!("SKIPPED: this ffmpeg could not build the fixture");
        return;
    }
    let plain = BgPipe::open(&clip, (4, 4), 0.0, 2, 1)
        .expect("open")
        .take()
        .expect("frame");
    let dimmed = BgPipe::open(&clip, (4, 4), 0.5, 2, 1)
        .expect("open")
        .take()
        .expect("frame");
    let mut expect = plain.clone();
    apply_dim(&mut expect, 0.5);
    assert_eq!(dimmed, expect);
    assert_ne!(
        dimmed, plain,
        "the fixture must not be black, else this proves nothing"
    );
    let _ = std::fs::remove_file(&clip);
}

fn encode_two_colour_clip(dest: &std::path::Path) -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=16x16:d=1:r=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=16x16:d=1:r=1",
            "-filter_complex",
            "[0:v][1:v]concat=n=2:v=1[v]",
            "-map",
            "[v]",
            "-c:v",
            "libx264rgb",
            "-crf",
            "0",
            "-pix_fmt",
            "rgb24",
            "-r",
            "1",
        ])
        .arg(dest)
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && dest.is_file()
}
