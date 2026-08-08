// Split from audio_mux.rs per repo convention (#[path] sibling test module).
use super::*;
use std::path::PathBuf;

/// A fresh temp folder for one test (never touches a real recording).
fn tmp_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("tcursor-mux-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn no_audio_renames_tmp_to_final_with_the_format_extension() {
    let folder = tmp_dir("mp4");
    let paths = ProjectPaths { folder: folder.clone() };
    let tmp = folder.join("tmp_export.mp4");
    std::fs::write(&tmp, b"fake video bytes").unwrap();
    mux(&tmp, &paths, Format::Mp4, 0, 0, 1.0, 1.0, 10_000).expect("mux");
    assert!(folder.join("final.mp4").exists());
    assert!(!tmp.exists());
    let _ = std::fs::remove_dir_all(&folder);
}

/// `Gif` can never carry audio, so `mux` must take the rename-only path even when mic audio
/// was actually recorded for this session - back-compat for `have_mic`/`have_sys` now being
/// gated on `format.supports_audio()`.
#[test]
fn gif_always_takes_the_no_audio_path_even_with_recorded_audio() {
    let folder = tmp_dir("gif");
    let paths = ProjectPaths { folder: folder.clone() };
    let tmp = folder.join("tmp_export.gif");
    std::fs::write(&tmp, b"fake gif bytes").unwrap();
    std::fs::write(paths.mic(), b"fake mic wav").unwrap();
    mux(&tmp, &paths, Format::Gif, 0, 0, 1.0, 1.0, 10_000).expect("mux");
    assert!(folder.join("final.gif").exists());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn webm_final_path_uses_the_webm_extension() {
    let folder = tmp_dir("webm");
    let paths = ProjectPaths { folder: folder.clone() };
    let tmp = folder.join("tmp_export.webm");
    std::fs::write(&tmp, b"fake webm bytes").unwrap();
    mux(&tmp, &paths, Format::WebM, 0, 0, 1.0, 1.0, 10_000).expect("mux");
    assert!(folder.join("final.webm").exists());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Both-tracks branch: the muxed audio must be capped to the (trimmed) video's duration via
/// `-t`, placed immediately before the output path - otherwise a 60s audio file muxed onto a
/// 10s trimmed video plays 60s of audio past the video's frozen last frame.
#[test]
fn both_tracks_args_cap_duration_with_t_immediately_before_the_output_path() {
    let tracks = [
        AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 1.0 },
        AudioTrack { path: Path::new("sys.wav"), shift_ms: 0, vol: 1.0 },
    ];
    let args = mux_args(Path::new("tmp.mp4"), &tracks, "aac", 10_000, Path::new("final.mp4"));
    assert_eq!(args.last().unwrap().to_str().unwrap(), "final.mp4");
    let t = args.iter().position(|a| a == "-t").expect("-t present");
    assert_eq!(args[t + 1].to_str().unwrap(), "10.000");
    assert_eq!(t + 2, args.len() - 1, "-t <val> must sit immediately before the output path: {args:?}");
}

/// Single-track branch: same duration cap requirement as the both-tracks branch.
#[test]
fn single_track_args_cap_duration_with_t_immediately_before_the_output_path() {
    let tracks = [AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 1.0 }];
    let args = mux_args(Path::new("tmp.mp4"), &tracks, "aac", 10_000, Path::new("final.mp4"));
    assert_eq!(args.last().unwrap().to_str().unwrap(), "final.mp4");
    let t = args.iter().position(|a| a == "-t").expect("-t present");
    assert_eq!(args[t + 1].to_str().unwrap(), "10.000");
    assert_eq!(t + 2, args.len() - 1, "-t <val> must sit immediately before the output path: {args:?}");
}
