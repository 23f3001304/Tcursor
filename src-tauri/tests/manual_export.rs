// Run explicitly:
//   cargo test --test manual_export -- --ignored --nocapture
// Finds the newest recording under %USERPROFILE%\Videos\TCursor (or the legacy
// CursorZoom) that has a video.mp4 + events.json, exports it, and verifies
// final.mp4 is produced.
use std::path::PathBuf;
use std::process::Command;

use cursor_zoom_lib::export::pipeline::exporter::export;
use cursor_zoom_lib::export::settings::ExportSettings;
use cursor_zoom_lib::session::paths::ProjectPaths;

#[test]
#[ignore]
fn exports_latest_recording_to_final_mp4() {
    let home = PathBuf::from(std::env::var("USERPROFILE").unwrap()).join("Videos");
    let videos = [home.join("TCursor"), home.join("CursorZoom")]
        .into_iter()
        .find(|d| newest_recording(d).is_some())
        .unwrap_or_else(|| home.join("TCursor"));
    let Some(name) = newest_recording(&videos) else {
        eprintln!("no recording with video.mp4 + events.json under {videos:?}; skipping");
        return;
    };
    eprintln!("exporting recording: {name}");

    let paths = ProjectPaths::new(&videos, &name);
    export(&paths, ExportSettings::default(), |p| eprintln!("progress {p}")).expect("export failed");

    let final_mp4 = paths.folder.join("final.mp4");
    let meta = std::fs::metadata(&final_mp4).expect("final.mp4 should exist");
    assert!(meta.len() > 0, "final.mp4 should be non-empty");
    eprintln!("wrote {} ({} bytes)", final_mp4.display(), meta.len());

    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=codec_type,width,height",
            "-of", "default=nw=1"])
        .arg(&final_mp4)
        .output()
        .expect("ffprobe final");
    eprintln!("ffprobe summary:\n{}", String::from_utf8_lossy(&out.stdout));
}

/// Newest `rec-*` directory that contains both video.mp4 and events.json.
fn newest_recording(videos: &std::path::Path) -> Option<String> {
    let mut best: Option<(std::time::SystemTime, String)> = None;
    for entry in std::fs::read_dir(videos).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("rec-") {
            continue;
        }
        let dir = entry.path();
        if !dir.join("video.mp4").exists() || !dir.join("events.json").exists() {
            continue;
        }
        let mtime = entry.metadata().and_then(|m| m.modified()).ok()?;
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, name));
        }
    }
    best.map(|(_, n)| n)
}
