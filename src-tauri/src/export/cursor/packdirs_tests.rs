use super::*;

/// A throwaway "resource dir" holding `assets/cursorpacks/<id>/pack.json` for each name.
fn fake_resources(tag: &str, packs: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("tcursor-packdirs-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (id, name) in packs {
        let dir = root.join("assets/cursorpacks").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pack.json"),
            format!(r#"{{"id":"{id}","name":"{name}","version":2}}"#)).unwrap();
    }
    root
}

#[test]
fn the_exe_relative_candidates_come_first_and_cover_dev() {
    let res = PathBuf::from("C:/some/resource/dir");
    let c = resource_candidates(BUNDLED_REL, Some(&res));
    assert!(c.len() >= 2, "at least the exe dir and one parent, got {c:?}");
    assert_eq!(c.last().unwrap(), &res.join("assets/cursorpacks"), "the Tauri dir is the fallback");
    assert!(c.iter().all(|p| p.ends_with("assets/cursorpacks")), "{c:?}");
    // The dev layout: the exe runs from src-tauri/target/<profile>, so two levels up from it IS
    // src-tauri, and `src-tauri/assets/cursorpacks` is the repo's own folder. Three exe-relative
    // candidates is what makes that reachable with no staging step.
    assert!(resource_candidates(BUNDLED_REL, None).len() >= 3, "{:?}", resource_candidates(BUNDLED_REL, None));
}

#[test]
fn a_resource_dir_with_no_cursorpacks_folder_yields_no_bundled_packs() {
    let empty = std::env::temp_dir().join("tcursor-packdirs-nothing-here");
    assert!(dirs_in(&empty.join("assets/cursorpacks")).is_empty());
}

#[test]
fn bundled_pack_dirs_lists_every_folder_alphabetically() {
    // Exercised through `dirs_in` against an explicit root: `bundled_root` memoizes per process,
    // so a test must not race other tests for that one slot.
    let root = fake_resources("listing", &[("zebra", "Zebra"), ("cat", "Cat"), ("aero", "Aero")]);
    let dirs = dirs_in(&root.join("assets/cursorpacks"));
    let names: Vec<String> =
        dirs.iter().map(|d| d.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, vec!["aero", "cat", "zebra"], "stable, alphabetical grid order");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_shipped_packs_resolve_from_the_running_test_binary() {
    // The whole point of the exe-relative candidates: `cargo test` runs from
    // src-tauri/target/<profile>/deps, so the repo's own assets folder must already be findable
    // with no resource dir and no staging. This is the same resolution `tauri dev` gets.
    let root = bundled_root(None).expect("bundled cursor packs must resolve from the test binary");
    assert!(root.join("cat").join("pack.json").is_file(), "expected the shipped packs at {root:?}");
    assert!(bundled_pack_dirs(None).len() >= 15, "all fifteen shipped packs should be listed");
    assert!(bundled_pack_dir("cat").is_some());
    assert!(bundled_pack_dir("definitely-not-a-pack").is_none());
}

#[test]
fn a_bundled_id_resolves_to_the_bundled_folder_not_an_import_path() {
    // One id is one pack: `resolve_pack_dir` must agree with `list_packs`'s bundled-wins order,
    // or the grid and the renderer would disagree about what "cat" means.
    assert_eq!(resolve_pack_dir("cat"), bundled_pack_dir("cat").unwrap());
    // An id nothing ships falls through to the imported location, whether or not it exists yet.
    assert_eq!(resolve_pack_dir("my_import"), pack_dir("my_import"));
}

#[test]
fn imported_packs_live_beside_the_config_file() {
    assert!(cursors_dir().ends_with("cursors"));
    assert!(cursors_dir().to_string_lossy().contains("TCursor"));
    assert_eq!(pack_dir("x").parent().unwrap(), cursors_dir());
}
