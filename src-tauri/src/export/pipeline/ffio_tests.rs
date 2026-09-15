use super::{crop_to_alpha, decode_file_cover, decode_image, decode_image_cover, StagedInput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const LEGACY_SHARED_TMP: &str = "cursorzoom_bg_src";

fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    let px: Vec<u8> = (0..w * h)
        .flat_map(|_| [rgb[0], rgb[1], rgb[2], 255])
        .collect();
    wr.write_image_data(&px).expect("png data");
    drop(wr);
    out
}

fn ffmpeg_present() -> bool {
    crate::process::proc::ffcmd("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

#[test]
fn crop_trims_to_content_and_reports_origin() {
    let mut buf = vec![0u8; 4 * 4 * 4];
    let i = (2 * 4 + 1) * 4;
    buf[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
    assert_eq!(
        crop_to_alpha(&buf, 4, 4),
        Some((vec![10, 20, 30, 255], 1, 1, 1, 2))
    );
    assert_eq!(crop_to_alpha(&vec![0u8; 4 * 4 * 4], 4, 4), None);
    assert_eq!(crop_to_alpha(&[], 0, 0), None);
}

#[test]
fn concurrently_staged_inputs_never_share_a_file() {
    let alive: Vec<_> = (0..8u8)
        .map(|i| {
            let bytes = vec![i; 64];
            let s = StagedInput::new(&bytes).expect("stage");
            (s, bytes)
        })
        .collect();
    let mut paths: Vec<_> = alive.iter().map(|(s, _)| s.path().to_path_buf()).collect();
    for (s, bytes) in &alive {
        assert_eq!(
            &std::fs::read(s.path()).expect("read staged"),
            bytes,
            "a staged input must still hold its OWN bytes while others are staged"
        );
    }
    let n = paths.len();
    paths.sort();
    paths.dedup();
    assert_eq!(
        paths.len(),
        n,
        "every concurrent staging must get a distinct path"
    );
}

#[test]
fn staged_input_is_deleted_on_drop() {
    let s = StagedInput::new(b"x").expect("stage");
    let p = s.path().to_path_buf();
    assert!(p.exists());
    drop(s);
    assert!(!p.exists(), "the staged file must not outlive its guard");
}

#[test]
fn decode_image_is_immune_to_a_clobber_of_the_legacy_shared_path() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let red = solid_png(4, 4, [255, 0, 0]);
    let blue = solid_png(4, 4, [0, 0, 255]);
    let legacy = std::env::temp_dir().join(LEGACY_SHARED_TMP);
    let stop = Arc::new(AtomicBool::new(false));
    let clobber = {
        let (stop, legacy, blue) = (stop.clone(), legacy.clone(), blue.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let _ = std::fs::write(&legacy, &blue);
            }
        })
    };
    let results: Vec<_> = (0..8).map(|_| decode_image(&red, 2, 2)).collect();
    stop.store(true, Ordering::Relaxed);
    let _ = clobber.join();
    let _ = std::fs::remove_file(&legacy);
    for r in results {
        let px = r.expect("decode must not fail because an unrelated file was being rewritten");
        assert!(
            px[2] > 200 && px[0] < 40,
            "decoded the wrong image: {:?}",
            &px[..4]
        );
    }
}

fn banded_png() -> Vec<u8> {
    let (w, h) = (4u32, 2u32);
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    let px: Vec<u8> = (0..w * h)
        .flat_map(|i| {
            if i % w == 0 || i % w == w - 1 {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            }
        })
        .collect();
    wr.write_image_data(&px).expect("png data");
    drop(wr);
    out
}

#[test]
fn cover_fit_crops_the_overflow_while_the_legacy_decode_still_stretches() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let src = banded_png();
    let cover = decode_image_cover(&src, 2, 2).expect("cover decode");
    assert_eq!(cover.len(), 2 * 2 * 4);
    for px in cover.chunks(4) {
        assert!(
            px[2] > 200 && px[1] < 60,
            "cover fit kept a column it should have cropped: {px:?}"
        );
    }
    let stretched = decode_image(&src, 2, 2).expect("stretch decode");
    assert!(
        stretched.chunks(4).any(|px| px[1] > 60),
        "the plain stretch must still fold the edges in"
    );
}

#[test]
fn the_file_input_decode_frames_exactly_like_the_in_memory_one() {
    if !ffmpeg_present() {
        eprintln!("SKIPPED: no ffmpeg on PATH");
        return;
    }
    let src = banded_png();
    let path = std::env::temp_dir().join(format!("tcursor_dfc_{}.png", std::process::id()));
    std::fs::write(&path, &src).expect("write fixture");
    let from_file = decode_file_cover(&path, 2, 2).expect("file cover decode");
    let from_bytes = decode_image_cover(&src, 2, 2).expect("bytes cover decode");
    assert_eq!(
        from_file, from_bytes,
        "the two cover decodes must be byte-identical"
    );
    assert!(
        decode_file_cover(&std::env::temp_dir().join("tcursor_no_such_file.png"), 2, 2).is_err(),
        "a missing file is an Err, which is what makes `background::build` fall back"
    );
    let _ = std::fs::remove_file(&path);
}
