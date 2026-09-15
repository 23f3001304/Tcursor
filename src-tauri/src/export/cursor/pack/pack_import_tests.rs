use super::*;

const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("tcursor_pack_import_test")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn import_rejects_a_path_that_is_not_a_directory() {
    let file = temp_dir("not_a_dir_parent").join("f.txt");
    std::fs::write(&file, b"hi").unwrap();
    let err = import_cursor_pack(file.to_string_lossy().to_string()).unwrap_err();
    assert!(err.contains("not a folder"), "got: {err}");
}

#[test]
fn import_rejects_a_folder_with_no_recognized_pngs() {
    let dir = temp_dir("no_recognized_pngs");
    std::fs::write(dir.join("readme.txt"), b"not a cursor").unwrap();
    let err = import_cursor_pack(dir.to_string_lossy().to_string()).unwrap_err();
    assert!(err.contains("no recognized cursor PNGs"), "got: {err}");
}

#[test]
fn collect_valid_sprites_only_keeps_recognized_names_with_real_png_bytes() {
    let dir = temp_dir("collect_mixed");
    std::fs::write(dir.join("arrow.png"), TINY_PNG).unwrap();
    std::fs::write(dir.join("hand.png"), b"not a png").unwrap();
    std::fs::write(dir.join("unknown_name.png"), TINY_PNG).unwrap();
    let found = collect_valid_sprites(&dir);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, "arrow.png");
    assert_eq!(found[0].1, TINY_PNG);
}

#[test]
fn read_and_validate_hotspots_ok_none_when_file_absent() {
    let dir = temp_dir("hotspots_absent");
    assert_eq!(read_and_validate_hotspots(&dir).unwrap(), None);
}

#[test]
fn read_and_validate_hotspots_ok_some_when_shape_matches() {
    let dir = temp_dir("hotspots_ok");
    std::fs::write(dir.join("hotspots.json"), br#"{"arrow": [0.1, 0.2]}"#).unwrap();
    let bytes = read_and_validate_hotspots(&dir).unwrap().expect("Some");
    let map: HashMap<String, (f32, f32)> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(map.get("arrow"), Some(&(0.1, 0.2)));
}

#[test]
fn read_and_validate_hotspots_errs_on_malformed_json() {
    let dir = temp_dir("hotspots_bad");
    std::fs::write(dir.join("hotspots.json"), b"{not json").unwrap();
    assert!(read_and_validate_hotspots(&dir).is_err());
}

#[test]
fn slugify_lowercases_and_replaces_punctuation() {
    assert_eq!(slugify("My Cool Pack!"), "my_cool_pack");
    assert_eq!(slugify("  "), "pack");
    assert_eq!(slugify(""), "pack");
    assert_eq!(slugify("Already_fine123"), "already_fine123");
}

#[test]
fn write_pack_creates_pngs_hotspots_and_meta() {
    let dir = temp_dir("write_pack_out").join("pack_dir");
    let sprites = vec![("arrow.png".to_string(), TINY_PNG.to_vec())];
    write_pack(
        &dir,
        "my_id",
        "My Name",
        &sprites,
        Some(br#"{"arrow":[0.2,0.3]}"#),
    )
    .unwrap();

    assert_eq!(std::fs::read(dir.join("arrow.png")).unwrap(), TINY_PNG);
    let hotspots: HashMap<String, (f32, f32)> =
        serde_json::from_slice(&std::fs::read(dir.join("hotspots.json")).unwrap()).unwrap();
    assert_eq!(hotspots.get("arrow"), Some(&(0.2, 0.3)));

    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("pack.json")).unwrap()).unwrap();
    assert_eq!(meta["id"], "my_id");
    assert_eq!(meta["name"], "My Name");
}

#[test]
fn write_pack_defaults_hotspots_to_empty_object_when_none_given() {
    let dir = temp_dir("write_pack_no_hotspots");
    write_pack(&dir, "id2", "Name2", &[], None).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("hotspots.json")).unwrap(),
        "{}"
    );
}
