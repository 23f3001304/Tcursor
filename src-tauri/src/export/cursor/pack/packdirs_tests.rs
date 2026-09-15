use super::*;

fn fake_resources(tag: &str, packs: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("tcursor-packdirs-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (id, name) in packs {
        let dir = root.join("assets/cursorpacks").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            format!(r#"{{"id":"{id}","name":"{name}","version":2}}"#),
        )
        .unwrap();
    }
    root
}

#[test]
fn the_exe_relative_candidates_come_first_and_cover_dev() {
    let res = PathBuf::from("C:/some/resource/dir");
    let c = resource_candidates(BUNDLED_REL, Some(&res));
    assert!(
        c.len() >= 2,
        "at least the exe dir and one parent, got {c:?}"
    );
    assert_eq!(
        c.last().unwrap(),
        &res.join("assets/cursorpacks"),
        "the Tauri dir is the fallback"
    );
    assert!(c.iter().all(|p| p.ends_with("assets/cursorpacks")), "{c:?}");
    assert!(
        resource_candidates(BUNDLED_REL, None).len() >= 3,
        "{:?}",
        resource_candidates(BUNDLED_REL, None)
    );
}

#[test]
fn a_resource_dir_with_no_cursorpacks_folder_yields_no_bundled_packs() {
    let empty = std::env::temp_dir().join("tcursor-packdirs-nothing-here");
    assert!(dirs_in(&empty.join("assets/cursorpacks")).is_empty());
}

#[test]
fn bundled_pack_dirs_lists_every_folder_alphabetically() {
    let root = fake_resources(
        "listing",
        &[("zebra", "Zebra"), ("cat", "Cat"), ("aero", "Aero")],
    );
    let dirs = dirs_in(&root.join("assets/cursorpacks"));
    let names: Vec<String> = dirs
        .iter()
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["aero", "cat", "zebra"],
        "stable, alphabetical grid order"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_shipped_packs_resolve_from_the_running_test_binary() {
    let root = bundled_root(None).expect("bundled cursor packs must resolve from the test binary");
    assert!(
        root.join("cat").join("pack.json").is_file(),
        "expected the shipped packs at {root:?}"
    );
    assert!(
        bundled_pack_dirs(None).len() >= 15,
        "all fifteen shipped packs should be listed"
    );
    assert!(bundled_pack_dir("cat").is_some());
    assert!(bundled_pack_dir("definitely-not-a-pack").is_none());
}

#[test]
fn a_bundled_id_resolves_to_the_bundled_folder_not_an_import_path() {
    assert_eq!(resolve_pack_dir("cat"), bundled_pack_dir("cat").unwrap());
    assert_eq!(resolve_pack_dir("my_import"), pack_dir("my_import"));
}

#[test]
fn imported_packs_live_beside_the_config_file() {
    assert!(cursors_dir().ends_with("cursors"));
    assert!(cursors_dir().to_string_lossy().contains("TCursor"));
    assert_eq!(pack_dir("x").parent().unwrap(), cursors_dir());
}
