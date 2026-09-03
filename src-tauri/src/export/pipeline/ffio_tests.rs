// Tests for `ffio.rs` - split into its own file purely for the size budget.
use super::{crop_to_alpha, decode_image, StagedInput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The one fixed path `decode_image` used to stage EVERY image at, regardless of caller. Named
/// here (and nowhere else in the crate any more) so the regression test below can clobber exactly
/// what the bug clobbered.
const LEGACY_SHARED_TMP: &str = "cursorzoom_bg_src";

/// A `w`x`h` solid-colour PNG, so a decode's result is checkable from any single pixel.
fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().expect("png header");
    let px: Vec<u8> = (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
    wr.write_image_data(&px).expect("png data");
    drop(wr);
    out
}

/// Is an `ffmpeg` we can spawn actually on this machine? The decode tests shell out for real
/// (that IS the thing under test); without one they report a skip rather than fail spuriously.
fn ffmpeg_present() -> bool {
    crate::win::sys::proc::ffcmd("ffmpeg").arg("-version").output().is_ok()
}

#[test]
fn crop_trims_to_content_and_reports_origin() {
    let mut buf = vec![0u8; 4 * 4 * 4]; // 4x4 transparent
    let i = (2 * 4 + 1) * 4; // one opaque pixel at (x=1, y=2)
    buf[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
    assert_eq!(crop_to_alpha(&buf, 4, 4), Some((vec![10, 20, 30, 255], 1, 1, 1, 2)));
    assert_eq!(crop_to_alpha(&vec![0u8; 4 * 4 * 4], 4, 4), None);
    assert_eq!(crop_to_alpha(&[], 0, 0), None); // zero dims: None, not a panic
}

/// Two staged inputs alive at once must never be the same file - the whole point of the guard.
/// Threads that stage concurrently each keep their own bytes readable for as long as they hold it.
#[test]
fn concurrently_staged_inputs_never_share_a_file() {
    let alive: Vec<_> = (0..8u8).map(|i| {
        let bytes = vec![i; 64];
        let s = StagedInput::new(&bytes).expect("stage");
        (s, bytes)
    }).collect();
    let mut paths: Vec<_> = alive.iter().map(|(s, _)| s.path().to_path_buf()).collect();
    for (s, bytes) in &alive {
        assert_eq!(&std::fs::read(s.path()).expect("read staged"), bytes,
            "a staged input must still hold its OWN bytes while others are staged");
    }
    let n = paths.len();
    paths.sort(); paths.dedup();
    assert_eq!(paths.len(), n, "every concurrent staging must get a distinct path");
}

/// A staged input is removed when its guard drops (the old inline cleanup leaked on `?`).
#[test]
fn staged_input_is_deleted_on_drop() {
    let s = StagedInput::new(b"x").expect("stage");
    let p = s.path().to_path_buf();
    assert!(p.exists());
    drop(s);
    assert!(!p.exists(), "the staged file must not outlive its guard");
}

/// REGRESSION (the giant-cursor background): an unrelated writer hammering the old fixed staging
/// path must not change what `decode_image` returns. Before the fix this WAS the staging path for
/// every caller, so a concurrent `decode_cursor` (the editor decodes the cursor pack at the same
/// moment a cold preview/export build decodes the mesh wallpaper) replaced the wallpaper's bytes
/// between the write and ffmpeg's read - and `preview_bg` returned a stretched cursor sprite.
#[test]
fn decode_image_is_immune_to_a_clobber_of_the_legacy_shared_path() {
    if !ffmpeg_present() { eprintln!("SKIPPED: no ffmpeg on PATH"); return; }
    let red = solid_png(4, 4, [255, 0, 0]);
    let blue = solid_png(4, 4, [0, 0, 255]);
    let legacy = std::env::temp_dir().join(LEGACY_SHARED_TMP);
    let stop = Arc::new(AtomicBool::new(false));
    let clobber = {
        let (stop, legacy, blue) = (stop.clone(), legacy.clone(), blue.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) { let _ = std::fs::write(&legacy, &blue); }
        })
    };
    let results: Vec<_> = (0..8).map(|_| decode_image(&red, 2, 2)).collect();
    stop.store(true, Ordering::Relaxed);
    let _ = clobber.join();
    let _ = std::fs::remove_file(&legacy);
    for r in results {
        let px = r.expect("decode must not fail because an unrelated file was being rewritten");
        // BGRA: red is [0, 0, 255, 255]; the clobbering blue would be [255, 0, 0, 255].
        assert!(px[2] > 200 && px[0] < 40, "decoded the wrong image: {:?}", &px[..4]);
    }
}
