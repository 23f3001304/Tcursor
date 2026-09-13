// The manual export benchmark, split out of `exporter.rs` for its line budget. `#[ignore]`d: it
// needs a REAL recording folder (`TCURSOR_REC`) and runs a full export, so it is opt-in only:
// `cargo test export_bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn export_bench() {
    let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
    let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(&folder) };
    let t = std::time::Instant::now();
    super::export(&paths, crate::export::settings::ExportSettings::default(), |_| {}).expect("export failed");
    eprintln!("export_bench: exported {folder} in {:?}", t.elapsed());
}
