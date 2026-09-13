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
    mux(&tmp, &paths, Format::Mp4, 0, 0, 1.0, 1.0, 10_000, &[]).expect("mux");
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
    mux(&tmp, &paths, Format::Gif, 0, 0, 1.0, 1.0, 10_000, &[]).expect("mux");
    assert!(folder.join("final.gif").exists());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn webm_final_path_uses_the_webm_extension() {
    let folder = tmp_dir("webm");
    let paths = ProjectPaths { folder: folder.clone() };
    let tmp = folder.join("tmp_export.webm");
    std::fs::write(&tmp, b"fake webm bytes").unwrap();
    mux(&tmp, &paths, Format::WebM, 0, 0, 1.0, 1.0, 10_000, &[]).expect("mux");
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
    let args = mux_args(Path::new("tmp.mp4"), &tracks, "aac", 10_000, Path::new("final.mp4"), &[]);
    assert_eq!(args.last().unwrap().to_str().unwrap(), "final.mp4");
    let t = args.iter().position(|a| a == "-t").expect("-t present");
    assert_eq!(args[t + 1].to_str().unwrap(), "10.000");
    assert_eq!(t + 2, args.len() - 1, "-t <val> must sit immediately before the output path: {args:?}");
}

/// Single-track branch: same duration cap requirement as the both-tracks branch.
#[test]
fn single_track_args_cap_duration_with_t_immediately_before_the_output_path() {
    let tracks = [AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 1.0 }];
    let args = mux_args(Path::new("tmp.mp4"), &tracks, "aac", 10_000, Path::new("final.mp4"), &[]);
    assert_eq!(args.last().unwrap().to_str().unwrap(), "final.mp4");
    let t = args.iter().position(|a| a == "-t").expect("-t present");
    assert_eq!(args[t + 1].to_str().unwrap(), "10.000");
    assert_eq!(t + 2, args.len() - 1, "-t <val> must sit immediately before the output path: {args:?}");
}

fn strs(args: &[OsString]) -> Vec<String> { args.iter().map(|a| a.to_string_lossy().into_owned()).collect() }

#[test]
fn without_segments_the_two_track_and_one_track_commands_are_the_pre_remap_ones() {
    let two = [AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 1.0 }, AudioTrack { path: Path::new("sys.wav"), shift_ms: 0, vol: 0.5 }];
    assert_eq!(strs(&mux_args(Path::new("tmp.mp4"), &two, "aac", 1500, Path::new("final.mp4"), &[])),
        ["-i", "tmp.mp4", "-i", "mic.wav", "-i", "sys.wav", "-filter_complex",
         "[1:a]volume=1[m];[2:a]volume=0.5[s];[m][s]amix=inputs=2:normalize=0[a]",
         "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac", "-t", "1.500", "final.mp4"]);
    let one = [AudioTrack { path: Path::new("mic.wav"), shift_ms: -250, vol: 1.0 }];
    let plain = [AudioSeg { start_s: 0.0, end_s: 1.5, factor: 1.0 }];
    assert_eq!(strs(&mux_args(Path::new("tmp.mp4"), &one, "aac", 1500, Path::new("final.mp4"), &plain)),
        ["-i", "tmp.mp4", "-ss", "0.250", "-i", "mic.wav", "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-af", "volume=1", "-t", "1.500", "final.mp4"]);
}

#[test]
fn segments_ride_the_mix_on_x_and_land_on_a_for_both_track_counts() {
    let segs = [AudioSeg { start_s: 0.0, end_s: 1.0, factor: 1.0 }, AudioSeg { start_s: 2.0, end_s: 3.0, factor: 2.0 }];
    let two = [AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 1.0 }, AudioTrack { path: Path::new("sys.wav"), shift_ms: 0, vol: 1.0 }];
    let args = strs(&mux_args(Path::new("tmp.mp4"), &two, "aac", 1500, Path::new("final.mp4"), &segs));
    let f = args.iter().position(|a| a == "-filter_complex").unwrap();
    assert!(args[f + 1].starts_with("[1:a]volume=1[m];[2:a]volume=1[s];[m][s]amix=inputs=2:normalize=0[x];[x]asplit=2[x0][x1];"), "{}", args[f + 1]);
    assert!(args[f + 1].ends_with("concat=n=2:v=0:a=1[a]"));
    let one = [AudioTrack { path: Path::new("mic.wav"), shift_ms: 0, vol: 0.8 }];
    let args = strs(&mux_args(Path::new("tmp.mp4"), &one, "aac", 1500, Path::new("final.mp4"), &segs));
    let f = args.iter().position(|a| a == "-filter_complex").unwrap();
    assert!(args[f + 1].starts_with("[1:a]volume=0.8[x];[x]asplit=2"), "{}", args[f + 1]);
    assert!(args.contains(&"[a]".to_string()) && !args.contains(&"-af".to_string()));
}

fn ffmpeg_present() -> bool { crate::win::sys::proc::ffcmd("ffmpeg").arg("-version").output().is_ok() }

#[test]
fn a_cut_and_a_double_speed_span_shorten_the_muxed_audio_to_match() {
    if !ffmpeg_present() { eprintln!("SKIPPED: no ffmpeg on PATH"); return; }
    let folder = tmp_dir("segs");
    let paths = ProjectPaths { folder: folder.clone() };
    let tmp = folder.join("tmp_export.mp4");
    // The video is ALREADY the output length (the exporter encodes only the plan's frames); `-t`
    // only caps the audio. A 3s video here would exercise stream-copy cutting instead, which is not
    // what the export does and lands on a B-frame boundary (1.7s) rather than 1.5s.
    let video = crate::win::sys::proc::ffcmd("ffmpeg").args(["-y", "-v", "error", "-f", "lavfi", "-i", "color=c=black:s=64x64:d=1.5:r=10", "-c:v", "libx264", "-pix_fmt", "yuv420p"])
        .arg(&tmp).status().map(|s| s.success()).unwrap_or(false);
    let audio = crate::win::sys::proc::ffcmd("ffmpeg").args(["-y", "-v", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=3"])
        .arg(paths.mic()).status().map(|s| s.success()).unwrap_or(false);
    if !(video && audio) { eprintln!("SKIPPED: could not build the lavfi fixtures"); return; }
    // keep 0..1s at 1x and 2..3s at 2x: 1.0 + 0.5 = 1.5s of output
    let segs = [AudioSeg { start_s: 0.0, end_s: 1.0, factor: 1.0 }, AudioSeg { start_s: 2.0, end_s: 3.0, factor: 2.0 }];
    mux(&tmp, &paths, Format::Mp4, 0, 0, 1.0, 1.0, 1500, &segs).expect("mux");
    let dur = crate::export::pipeline::ffio::probe_duration(&folder.join("final.mp4")).expect("probe final");
    assert!((dur - 1.5).abs() < 0.12, "muxed duration {dur}s, expected about 1.5s");
    let _ = std::fs::remove_dir_all(&folder);
}
