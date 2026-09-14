// The manual export benchmark, split out of `exporter.rs` for its line budget. `#[ignore]`d: it
// needs a REAL recording folder (`TCURSOR_REC`) and runs a full export, so it is opt-in only:
// `cargo test export_bench -- --ignored --nocapture`.
/// One composited frame of a real recording to `%TEMP%/tcursor-preview-frame.png`, through the
/// editor's own `render_preview` (the GPU compositor, no video encoder): `TCURSOR_REC` names the
/// folder, `TCURSOR_MS` the instant (default 1500). Opt-in like the bench:
/// `cargo test preview_frame_bench -- --ignored --nocapture`. Made for 2026-09-14's odd-sized
/// capture, whose export slid and sheared - a look at one frame says whether a decode is sound
/// without occupying the machine's hardware encoder for a full export.
#[test]
#[ignore]
fn preview_frame_bench() {
    let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
    let ms: u32 = std::env::var("TCURSOR_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(&folder) };
    let png = crate::export::preview::render_preview(&paths, ms).expect("render failed");
    let out = std::env::temp_dir().join("tcursor-preview-frame.png");
    std::fs::write(&out, png).expect("write png");
    eprintln!("preview_frame_bench: frame at {ms}ms of {folder} -> {}", out.display());
}

#[test]
#[ignore]
fn export_bench() {
    let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
    let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(&folder) };
    let t = std::time::Instant::now();
    super::export(&paths, crate::export::settings::ExportSettings::default(), |_| {}).expect("export failed");
    eprintln!("export_bench: exported {folder} in {:?}", t.elapsed());
}
