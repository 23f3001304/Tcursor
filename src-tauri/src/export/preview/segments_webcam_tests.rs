use super::*;
use crate::session::sync::Segment;

fn part(file: &str, start_ms: u64, dur_ms: u64, w: u32, h: u32) -> Part {
    Part {
        file: file.into(),
        start_ms,
        dur_ms,
        w,
        h,
    }
}

fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn one_segment_alone_has_nothing_to_merge() {
    assert_eq!(
        merge_args(&[part("a.webm", 0, 500, 640, 480)], Path::new("out.webm")),
        None
    );
    assert_eq!(merge_args(&[], Path::new("out.webm")), None);
}

#[test]
fn a_gap_is_the_dark_stretch_between_one_segment_ending_and_the_next_starting() {
    let parts = [
        part("a.webm", 0, 500, 640, 480),
        part("b.webm", 700, 400, 640, 480),
        part("c.webm", 1300, 300, 640, 480),
    ];
    assert_eq!(gaps(&parts), vec![0, 200, 200]);
}

#[test]
fn a_stamp_inside_the_previous_segment_closes_the_gap_instead_of_going_negative() {
    let parts = [
        part("a.webm", 0, 900, 640, 480),
        part("b.webm", 800, 400, 640, 480),
    ];
    assert_eq!(gaps(&parts), vec![0, 0]);
    let three = [
        part("a.webm", 0, 900, 640, 480),
        part("b.webm", 800, 400, 640, 480),
        part("c.webm", 1500, 100, 640, 480),
    ];
    assert_eq!(gaps(&three), vec![0, 0, 200]);
}

#[test]
fn two_segments_with_a_gap_and_a_size_mismatch_pad_into_the_first_segments_size() {
    let parts = [
        part("webcam.webm", 0, 500, 640, 480),
        part("webcam_2.webm", 1000, 500, 1280, 720),
    ];
    let args = merge_args(&parts, Path::new("tmp.webm")).expect("two parts merge");
    let joined = args.join(" ");
    assert!(
        joined.contains("-i webcam.webm -i webcam_2.webm"),
        "{joined}"
    );
    let graph = &args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
    assert!(
        graph.contains("color=c=black:size=640x480:rate=30:duration=0.500"),
        "{graph}"
    );
    assert!(
        graph.contains("[v0][g1][v1]concat=n=3:v=1:a=0[out]"),
        "{graph}"
    );
    assert!(
        graph.contains("[1:v]scale=640:480:force_original_aspect_ratio=decrease,pad=640:480:"),
        "{graph}"
    );
    assert!(joined.contains("-c:v libvpx-vp9"), "{joined}");
    assert!(joined.ends_with("tmp.webm"), "{joined}");
}

#[test]
fn back_to_back_segments_need_no_black_at_all() {
    let parts = [
        part("webcam.webm", 0, 500, 640, 480),
        part("webcam_2.webm", 500, 500, 640, 480),
    ];
    let graph = {
        let args = merge_args(&parts, Path::new("tmp.webm")).unwrap();
        args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1].clone()
    };
    assert!(!graph.contains("color=c=black"), "{graph}");
    assert!(graph.contains("[v0][v1]concat=n=2:v=1:a=0[out]"), "{graph}");
}

#[test]
fn a_take_without_a_switch_is_untouched() {
    let dir = std::env::temp_dir().join(format!("tcursor_wcmerge_none_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = ProjectPaths {
        folder: dir.clone(),
    };
    std::fs::write(paths.webcam(), b"not really a webm").unwrap();
    assert_eq!(merge_webcam_segments(&paths, &SyncLog::default()), Ok(()));
    assert_eq!(std::fs::read(paths.webcam()).unwrap(), b"not really a webm");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_real_segments_merge_into_one_file_whose_duration_includes_the_gap() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!("tcursor_wcmerge_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let paths = ProjectPaths {
        folder: dir.clone(),
    };
    let gen = |out: &Path, src: &str, size: &str| {
        let ok = crate::process::proc::ffcmd("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("{src}=size={size}:rate=30:duration=0.5"),
            ])
            .args([
                "-c:v",
                "libvpx-vp9",
                "-deadline",
                "realtime",
                "-cpu-used",
                "8",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(out)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "generating {out:?} failed");
    };
    gen(&paths.webcam(), "testsrc", "320x240");
    gen(&dir.join("webcam_2.webm"), "smptebars", "640x360");
    let sync = SyncLog {
        webcam_segments: vec![Segment {
            path: "webcam_2.webm".into(),
            start_ms: 1000,
        }],
        ..Default::default()
    };

    merge_webcam_segments(&paths, &sync).expect("the merge runs");

    let secs = probe_duration(&paths.webcam()).unwrap();
    assert!(
        (secs - 1.5).abs() < 0.2,
        "merged duration {secs}s is not 0.5 + 0.5 gap + 0.5"
    );
    assert_eq!(
        probe_dims(&paths.webcam()).unwrap(),
        (320, 240),
        "the first segment's size wins"
    );
    assert!(
        dir.join("segments").join("webcam.webm").exists(),
        "the originals are kept for a re-merge"
    );
    assert!(dir.join("segments").join("webcam_2.webm").exists());
    assert!(
        !dir.join("webcam_2.webm").exists(),
        "the extra segment is gone from the project root"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
