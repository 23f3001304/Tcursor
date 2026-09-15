use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn tempdir() -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "tcursor_bga_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&p).expect("create tempdir");
    p
}
fn drop_dir(p: &Path) {
    let _ = std::fs::remove_dir_all(p);
}
fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn extensions_map_to_the_two_kinds_and_nothing_else() {
    for e in ["png", "JPG", "jpeg", "webp"] {
        assert_eq!(asset_kind_for(e), Some("image"), "{e}");
    }
    for e in ["gif", "mp4", "WEBM", "mov"] {
        assert_eq!(asset_kind_for(e), Some("video"), "{e}");
    }
    for e in ["exe", "svg", "mkv", ""] {
        assert_eq!(asset_kind_for(e), None, "{e}");
    }
}

#[test]
fn a_name_collision_gets_a_numeric_suffix_not_an_overwrite() {
    let dir = tempdir();
    assert_eq!(unique_name(&dir, "loop.mp4"), "loop.mp4");
    std::fs::write(dir.join("loop.mp4"), b"x").unwrap();
    assert_eq!(unique_name(&dir, "loop.mp4"), "loop_2.mp4");
    std::fs::write(dir.join("loop_2.mp4"), b"x").unwrap();
    assert_eq!(unique_name(&dir, "loop.mp4"), "loop_3.mp4");
    std::fs::write(dir.join("noext"), b"x").unwrap();
    assert_eq!(
        unique_name(&dir, "noext"),
        "noext_2",
        "a file with no extension still dedupes"
    );
    drop_dir(&dir);
}

#[test]
fn a_relative_path_must_stay_inside_the_project() {
    let dir = tempdir();
    std::fs::create_dir_all(dir.join("background")).unwrap();
    std::fs::write(dir.join("background/a.png"), b"x").unwrap();
    assert!(asset_path(&dir, "background/a.png").is_some());
    assert_eq!(
        asset_path(&dir, "background/../../secrets.png"),
        None,
        "traversal must be refused"
    );
    assert_eq!(
        asset_path(&dir, "C:/Windows/win.ini"),
        None,
        "an absolute path is never an asset"
    );
    assert_eq!(
        asset_path(&dir, "background/missing.png"),
        None,
        "a deleted asset reads as absent"
    );
    assert_eq!(asset_path(&dir, ""), None);
    drop_dir(&dir);
}

#[test]
fn the_thumb_path_is_derived_not_stored() {
    assert_eq!(
        thumb_rel("background/my clip.mp4"),
        "background/.thumbs/my clip.mp4.jpg"
    );
    assert_eq!(thumb_rel("a.png"), ".thumbs/a.png.jpg");
}

#[test]
fn import_refuses_what_it_cannot_render() {
    let (proj, src) = (tempdir(), tempdir());
    let bad = src.join("notes.txt");
    std::fs::write(&bad, b"x").unwrap();
    let e = import_blocking(&proj, &bad).unwrap_err();
    assert!(
        e.contains("png") && e.contains("mov"),
        "the error must name what IS accepted: {e}"
    );
    let e = import_blocking(&proj, &src.join("gone.png")).unwrap_err();
    assert!(e.contains("not a file"), "{e}");
    assert!(
        !proj.join("background").exists(),
        "a refused import must not leave a folder behind"
    );
    drop_dir(&proj);
    drop_dir(&src);
}

#[test]
fn import_copies_the_file_in_keeps_its_name_and_returns_a_relative_path() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let (proj, src) = (tempdir(), tempdir());
    let png = src.join("hero.png");
    std::fs::write(&png, solid_png(8, 4)).unwrap();
    let info = import_blocking(&proj, &png).expect("import");
    assert_eq!(
        info.rel_path, "background/hero.png",
        "always relative, always forward slashes"
    );
    assert_eq!(info.kind, "image");
    assert_eq!(
        (info.width, info.height),
        (8, 4),
        "probed from the real file"
    );
    assert_eq!(info.duration_ms, None, "a still has no duration");
    assert!(
        proj.join("background/hero.png").is_file(),
        "the file is IN the project now"
    );
    assert!(
        proj.join("background/.thumbs/hero.png.jpg").is_file(),
        "a thumbnail rides along"
    );
    let again = import_blocking(&proj, &png).expect("import 2");
    assert_eq!(again.rel_path, "background/hero_2.png");
    assert_eq!(
        asset_path(&proj, &info.rel_path),
        Some(proj.join("background").join("hero.png"))
    );
    drop_dir(&proj);
    drop_dir(&src);
}

#[test]
fn info_reports_a_missing_asset_as_none_and_remove_is_idempotent() {
    let proj = tempdir();
    std::fs::create_dir_all(proj.join("background/.thumbs")).unwrap();
    std::fs::write(proj.join("background/a.png"), b"x").unwrap();
    std::fs::write(proj.join("background/.thumbs/a.png.jpg"), b"x").unwrap();
    remove_blocking(&proj, "background/a.png").expect("remove");
    assert!(!proj.join("background/a.png").exists());
    assert!(
        !proj.join("background/.thumbs/a.png.jpg").exists(),
        "the thumb goes with it"
    );
    remove_blocking(&proj, "background/a.png").expect("a second remove is not an error");
    assert!(
        info_blocking(&proj, "background/a.png").is_none(),
        "a gone asset has no info"
    );
    assert!(remove_blocking(&proj, "../../anything.png").is_err());
    drop_dir(&proj);
}

fn solid_png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    wr.write_image_data(&vec![255u8; (w * h * 4) as usize])
        .expect("png data");
    drop(wr);
    out
}
