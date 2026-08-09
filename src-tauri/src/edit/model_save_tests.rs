// Atomic-save + corrupt-file-preservation tests, split out of model_tests.rs so that file stays
// under the size limit.
use super::*;

fn corrupt_path(edit_path: &std::path::Path) -> PathBuf {
    edit_path.with_file_name(format!(
        "{}.corrupt",
        edit_path.file_name().unwrap().to_string_lossy()
    ))
}

/// A crash mid-write must never leave a half-written `.tmp`/`.part-*` sibling behind after a
/// successful `save` - only the real target file should exist in the directory afterward.
#[test]
fn save_leaves_no_tmp_sibling_on_success() {
    let p = tmp_path("edit_model_no_tmp_leftover.json");
    let _ = std::fs::remove_file(&p);
    sample_doc().save(&p).unwrap();
    let dir = p.parent().unwrap();
    let target_name = p.file_name().unwrap().to_string_lossy().into_owned();
    let stray: Vec<String> = std::fs::read_dir(dir).unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != &target_name && n.contains(&target_name))
        .collect();
    assert!(stray.is_empty(), "leftover tmp files: {:?}", stray);
    let _ = std::fs::remove_file(&p);
}

/// `save` must atomically REPLACE an existing target (the common case - every edit op saves over
/// the same `edit.json`), not merely succeed when the file is absent.
#[test]
fn save_overwrites_an_existing_file() {
    let p = tmp_path("edit_model_overwrite.json");
    sample_doc().save(&p).unwrap();
    let mut doc2 = sample_doc();
    doc2.trim.in_ms = 999;
    doc2.save(&p).unwrap();
    let loaded = EditDoc::load(&p).unwrap();
    assert_eq!(loaded.trim.in_ms, 999);
    let _ = std::fs::remove_file(&p);
}

/// A parse failure must not look like "no file": `load_or_seed` would silently reseed and the
/// user's edit would vanish. Instead the corrupt bytes are preserved on disk (renamed aside) so
/// nothing is lost, and `load` returns `None` so the caller's reseed path still runs.
#[test]
fn load_on_truncated_json_returns_none_and_preserves_original_bytes() {
    let p = tmp_path("edit_model_truncated.json");
    let corrupt = corrupt_path(&p);
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(&corrupt);
    let truncated: &[u8] = br#"{"version": 1,"#;
    std::fs::write(&p, truncated).unwrap();

    let loaded = EditDoc::load(&p);

    assert!(loaded.is_none());
    assert!(!p.exists(), "corrupt file should be moved aside, not left in place");
    assert_eq!(std::fs::read(&corrupt).unwrap(), truncated);
    let _ = std::fs::remove_file(&corrupt);
}

/// A second corrupt file must not fail to be preserved just because an older `.corrupt` sibling
/// (from a previous crash) is already sitting there.
#[test]
fn load_on_truncated_json_overwrites_an_older_corrupt_file() {
    let p = tmp_path("edit_model_truncated_twice.json");
    let corrupt = corrupt_path(&p);
    std::fs::write(&corrupt, b"stale corrupt from last time").unwrap();
    let truncated: &[u8] = br#"{"broken"#;
    std::fs::write(&p, truncated).unwrap();

    let loaded = EditDoc::load(&p);

    assert!(loaded.is_none());
    assert_eq!(std::fs::read(&corrupt).unwrap(), truncated);
    let _ = std::fs::remove_file(&corrupt);
}
