use super::*;

fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn a_cut_and_a_double_speed_span_shorten_the_muxed_audio_to_match() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let folder = tmp_dir("segs");
    let paths = ProjectPaths {
        folder: folder.clone(),
    };
    let tmp = folder.join("tmp_export.mp4");
    let video = crate::process::proc::ffcmd("ffmpeg")
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=1.5:r=10",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&tmp)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let audio = crate::process::proc::ffcmd("ffmpeg")
        .args([
            "-y",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
        ])
        .arg(paths.mic())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !(video && audio) {
        eprintln!("SKIPPED: could not build the lavfi fixtures");
        return;
    }
    let segs = [
        AudioSeg {
            start_s: 0.0,
            end_s: 1.0,
            factor: 1.0,
        },
        AudioSeg {
            start_s: 2.0,
            end_s: 3.0,
            factor: 2.0,
        },
    ];
    mux(&tmp, &paths, Format::Mp4, 0, 0, 1.0, 1.0, 1500, &segs).expect("mux");
    let dur = crate::export::pipeline::ffio::probe_duration(&folder.join("final.mp4"))
        .expect("probe final");
    assert!(
        (dur - 1.5).abs() < 0.12,
        "muxed duration {dur}s, expected about 1.5s"
    );
    let _ = std::fs::remove_dir_all(&folder);
}
