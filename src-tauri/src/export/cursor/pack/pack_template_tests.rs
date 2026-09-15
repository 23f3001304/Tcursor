use super::*;
use crate::events::track::cursortype::CursorType;
use crate::export::cursor::pack::{read_meta, sprite_sources_from_dir};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("tcursor_pack_template_test")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn rejects_a_path_that_is_not_a_directory() {
    let file = temp_dir("not_a_dir_parent").join("f.txt");
    std::fs::write(&file, b"hi").unwrap();
    let err = create_pack_template(file.to_string_lossy().to_string()).unwrap_err();
    assert!(err.contains("not a folder"), "got: {err}");
}

#[test]
fn unique_template_dir_suffixes_on_collision() {
    let root = temp_dir("collision");
    assert_eq!(unique_template_dir(&root), root.join("my-pack"));
    std::fs::create_dir_all(root.join("my-pack")).unwrap();
    assert_eq!(unique_template_dir(&root), root.join("my-pack-2"));
    std::fs::create_dir_all(root.join("my-pack-2")).unwrap();
    assert_eq!(unique_template_dir(&root), root.join("my-pack-3"));
}

#[test]
fn create_pack_template_writes_a_folder_the_real_loader_reads_back() {
    let root = temp_dir("real_loader");
    let created = create_pack_template(root.to_string_lossy().to_string()).unwrap();
    let dir = PathBuf::from(&created);
    assert_eq!(dir, root.join("my-pack"));

    let rows = sprite_sources_from_dir(&dir);
    assert_eq!(rows.len(), SPRITES.len());
    let mut kinds: Vec<CursorType> = rows.iter().map(|(k, ..)| *k).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    let mut expected: Vec<CursorType> = SPRITES.iter().map(|&(k, ..)| k).collect();
    expected.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(kinds, expected, "all nine cursor states must be listed");
    for (kind, bytes, _hot) in &rows {
        let on_disk = std::fs::read(dir.join(kind_filename(*kind))).unwrap();
        assert_eq!(
            bytes, &on_disk,
            "{kind:?} sprite must be read from the written file, not a fallback"
        );
    }

    let meta = read_meta(&dir).expect("pack.json must parse");
    assert_eq!(meta.id, "my-pack");
    assert_eq!(meta.name, "My Pack");
    assert!(
        meta.busy.is_some(),
        "the template must declare a busy animation"
    );

    let hotspots: std::collections::HashMap<String, (f32, f32)> =
        serde_json::from_slice(&std::fs::read(dir.join("hotspots.json")).unwrap()).unwrap();
    assert_eq!(hotspots.len(), SPRITES.len());

    let readme = std::fs::read_to_string(dir.join("README.txt")).unwrap();
    assert!(readme.contains("pack.json"));
    assert!(readme.contains("hotspots.json"));
    assert!(readme.contains("busy"));
    assert!(readme.is_ascii(), "README.txt must be plain ASCII");
}

#[test]
fn creating_a_second_template_in_the_same_folder_does_not_clobber_the_first() {
    let root = temp_dir("no_clobber");
    let first = create_pack_template(root.to_string_lossy().to_string()).unwrap();
    let second = create_pack_template(root.to_string_lossy().to_string()).unwrap();
    assert_ne!(first, second);
    assert!(PathBuf::from(&first).is_dir());
    assert!(PathBuf::from(&second).is_dir());
}
