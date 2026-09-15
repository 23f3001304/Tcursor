use super::*;

#[test]
fn choose_dir_picks_first_with_the_binary() {
    let base = std::env::temp_dir().join(format!("tcursor-ff-{}", std::process::id()));
    let (a, b) = (base.join("a"), base.join("b"));
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(b.join("ffmpeg.exe"), b"x").unwrap();
    let candidates = vec![a.clone(), b.clone()];
    assert_eq!(choose_dir(&candidates, "ffmpeg.exe"), Some(&b));
    assert_eq!(choose_dir(&candidates, "nope.exe"), None);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn candidates_lead_with_exe_dir_then_resource_dir() {
    let candidates = ffmpeg_candidates(Some(Path::new("C:/res")));
    assert_eq!(
        candidates[candidates.len() - 2],
        Path::new("C:/res").join("resources")
    );
    assert_eq!(
        candidates[candidates.len() - 1],
        Path::new("C:/res").to_path_buf()
    );
}

#[test]
fn corrupt_sibling_appends_the_suffix_in_the_same_directory() {
    let p = Path::new("C:/proj/edit.json");
    assert_eq!(corrupt_sibling(p), Path::new("C:/proj/edit.json.corrupt"));
}

#[test]
fn preserve_corrupt_moves_the_bad_file_aside_and_leaves_original_bytes_intact() {
    let dir = std::env::temp_dir().join(format!("tcursor-corrupt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    std::fs::write(&path, b"not json").unwrap();
    preserve_corrupt(&path, &"boom");
    assert!(
        !path.exists(),
        "the bad file must be moved OFF the original path"
    );
    let corrupt = corrupt_sibling(&path);
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"not json");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preserve_corrupt_overwrites_a_stale_corrupt_sibling_from_an_earlier_crash() {
    let dir = std::env::temp_dir().join(format!("tcursor-corrupt2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    std::fs::write(&path, b"fresh-bad").unwrap();
    std::fs::write(corrupt_sibling(&path), b"stale-from-a-prior-crash").unwrap();
    preserve_corrupt(&path, &"boom");
    assert_eq!(std::fs::read(corrupt_sibling(&path)).unwrap(), b"fresh-bad");
    let _ = std::fs::remove_dir_all(&dir);
}
