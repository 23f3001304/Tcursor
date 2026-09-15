use super::*;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("tcursor_project_commands_test")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn resolves_the_containing_folder_when_a_manifest_is_present() {
    let dir = temp_dir("with_manifest");
    let manifest_path = dir.join("project.tcursor");
    ProjectManifest::new(1920, 1080)
        .save(&manifest_path)
        .unwrap();

    let folder = folder_from_manifest_path(&manifest_path).unwrap();
    assert_eq!(Path::new(&folder), dir);
}

#[test]
fn falls_back_when_the_folder_has_no_manifest_at_all() {
    let dir = temp_dir("no_manifest");
    let hypothetical_manifest_path = dir.join("project.tcursor");
    let folder = folder_from_manifest_path(&hypothetical_manifest_path).unwrap();
    assert_eq!(Path::new(&folder), dir);
}

#[test]
fn errors_when_the_parent_folder_does_not_exist() {
    let missing = std::env::temp_dir()
        .join("tcursor_project_commands_test")
        .join("__does_not_exist__")
        .join("project.tcursor");
    assert!(folder_from_manifest_path(&missing).is_err());
}

#[test]
fn launch_project_from_argv_resolves_a_tcursor_path() {
    let dir = temp_dir("argv_launch");
    let manifest_path = dir.join("project.tcursor");
    ProjectManifest::new(640, 480).save(&manifest_path).unwrap();

    let args = vec![
        "TCursor.exe".to_string(),
        manifest_path.to_string_lossy().into_owned(),
    ];
    let folder = launch_project_from_argv(args.into_iter()).unwrap();
    assert_eq!(Path::new(&folder), dir);
}

#[test]
fn launch_project_from_argv_is_case_insensitive_about_the_extension() {
    let dir = temp_dir("argv_launch_caps");
    let manifest_path = dir.join("PROJECT.TCURSOR");
    ProjectManifest::new(1, 1).save(&manifest_path).unwrap();

    let args = vec![
        "TCursor.exe".to_string(),
        manifest_path.to_string_lossy().into_owned(),
    ];
    assert!(launch_project_from_argv(args.into_iter()).is_some());
}

#[test]
fn launch_project_from_argv_ignores_missing_or_non_tcursor_args() {
    assert!(launch_project_from_argv(vec!["TCursor.exe".to_string()].into_iter()).is_none());
    assert!(launch_project_from_argv(
        vec!["TCursor.exe".to_string(), "--devtools".to_string()].into_iter()
    )
    .is_none());
}
