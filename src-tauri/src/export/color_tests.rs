use super::*;

/// BT.601 limited-range anchors: Y=16 is black, Y=235 is white, neutral chroma (128,128).
#[test]
fn luma_endpoints_map_to_black_and_white() {
    assert_eq!(yuv_to_rgb(16.0, 128.0, 128.0), (0, 0, 0));
    assert_eq!(yuv_to_rgb(235.0, 128.0, 128.0), (255, 255, 255));
}

/// A solid color has uniform chroma, so 4:2:0 subsampling is lossless and BGRA -> nv12 -> BGRA
/// round-trips within rounding error (a few LSBs). Exercises the exact plane layout the GPU
/// upload + shader rely on.
#[test]
fn solid_color_roundtrips_within_rounding() {
    for px in [[20u8, 60, 200, 255], [200, 200, 200, 255], [0, 128, 255, 255], [10, 10, 10, 255]] {
        let (w, h) = (8u32, 6u32);
        let bgra: Vec<u8> = std::iter::repeat(px).take((w * h) as usize).flatten().collect();
        let nv12 = bgra_to_nv12(&bgra, w, h);
        assert_eq!(nv12.len(), (w * h + (w * h) / 2) as usize);
        let back = nv12_to_bgra(&nv12, w, h);
        for (i, (a, b)) in bgra.iter().zip(back.iter()).enumerate() {
            if i % 4 == 3 { continue; } // alpha is forced to 255
            assert!((*a as i32 - *b as i32).abs() <= 4, "px {px:?} chan {} {a} vs {b}", i % 4);
        }
    }
}

/// nv12_to_bgra must reproduce ffmpeg's own `-pix_fmt bgra` output (what the old export path fed the
/// compositor), so switching the decoder to nv12 changes no exported pixels. Needs a real recording:
///   TCURSOR_REC=<folder> cargo test --lib export::color::tests::matches_ffmpeg -- --ignored --nocapture
#[test]
#[ignore]
fn matches_ffmpeg_bgra() {
    use std::process::Command;
    let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
    let video = std::path::Path::new(&folder).join("video.mp4");
    let (w, h) = crate::export::pipeline::ffio::probe_dims(&video).expect("probe dims");
    let grab = |pix: &str| -> Vec<u8> {
        Command::new("ffmpeg").args(["-v", "error", "-i"]).arg(&video)
            .args(["-vframes", "1", "-f", "rawvideo", "-pix_fmt", pix, "pipe:1"])
            .output().expect("ffmpeg").stdout
    };
    let reference = grab("bgra");
    let ours = nv12_to_bgra(&grab("nv12"), w, h);
    assert_eq!(reference.len(), ours.len(), "size mismatch {}x{}", w, h);
    let (mut sum, mut max) = (0u64, 0i32);
    for (i, (a, b)) in reference.iter().zip(ours.iter()).enumerate() {
        if i % 4 == 3 { continue; }
        let d = (*a as i32 - *b as i32).abs();
        sum += d as u64; max = max.max(d);
    }
    let mean = sum as f64 / (reference.len() as f64 * 0.75);
    eprintln!("nv12_to_bgra vs ffmpeg: meanAbsDiff={mean:.3} maxAbsDiff={max}");
    assert!(max <= 4, "max abs diff {max} too high - color matrix mismatch");
}
