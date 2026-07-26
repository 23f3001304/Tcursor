// Split from manifest.rs per repo convention (#[path] sibling test module).
use super::*;

fn temp_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("tcursor_manifest_test");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(name)
}

#[test]
fn round_trips_through_json() {
    let path = temp_path("roundtrip.tcursor");
    let m = ProjectManifest::new(1920, 1080);
    m.save(&path).unwrap();

    let back = ProjectManifest::load(&path).unwrap();
    assert_eq!(back.version, MANIFEST_VERSION);
    assert_eq!(back.source_w, 1920);
    assert_eq!(back.source_h, 1080);
    assert_eq!(back.app_version, env!("CARGO_PKG_VERSION"));
    assert!(!back.preprocessed);
    assert!(back.created_unix_ms > 0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_errs_when_the_file_is_missing() {
    let path = temp_path("does_not_exist.tcursor");
    let _ = std::fs::remove_file(&path);
    assert!(ProjectManifest::load(&path).is_err());
}

#[test]
fn load_errs_when_the_file_is_not_valid_json() {
    let path = temp_path("corrupt.tcursor");
    std::fs::write(&path, b"not json").unwrap();
    assert!(ProjectManifest::load(&path).is_err());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_or_default_falls_back_to_an_unknown_source_manifest_when_missing() {
    // Simulates an existing recording made before this feature: no project.tcursor on disk at
    // all. Resolution must still succeed (back-compat), with source dims that clearly read as
    // "unknown" rather than a fabricated real resolution.
    let path = temp_path("missing_for_fallback.tcursor");
    let _ = std::fs::remove_file(&path);

    let m = ProjectManifest::load_or_default(&path);
    assert_eq!(m.source_w, 0);
    assert_eq!(m.source_h, 0);
    assert!(!m.preprocessed);
    assert_eq!(m.version, MANIFEST_VERSION);
}

#[test]
fn load_or_default_falls_back_when_the_file_is_corrupt_too() {
    let path = temp_path("corrupt_for_fallback.tcursor");
    std::fs::write(&path, b"{ not: valid").unwrap();
    let m = ProjectManifest::load_or_default(&path);
    assert_eq!(m.source_w, 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_or_default_returns_the_real_manifest_when_present() {
    let path = temp_path("present_for_fallback.tcursor");
    ProjectManifest::new(640, 480).save(&path).unwrap();
    let m = ProjectManifest::load_or_default(&path);
    assert_eq!(m.source_w, 640);
    assert_eq!(m.source_h, 480);
    let _ = std::fs::remove_file(&path);
}
