use super::*;
use crate::export::grade::{apply_px, GradeParams};
use crate::settings::grade::GradePreset;

fn params(p: GradePreset) -> GradeParams {
    let (exposure, contrast, vignette) = crate::export::grade::seed_of(p);
    crate::export::grade::params_of(&crate::settings::grade::GradeSettings {
        preset: p,
        exposure,
        contrast,
        vignette,
    })
    .unwrap()
}

fn frame(w: u32, h: u32) -> Vec<u8> {
    let mut v = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            v[i] = (x * 3) as u8;
            v[i + 1] = (y * 5) as u8;
            v[i + 2] = ((x + y) * 2) as u8;
        }
    }
    v
}

#[test]
fn the_loop_agrees_with_apply_px_at_every_pixel() {
    let (w, h) = (17u32, 13u32);
    let p = params(GradePreset::Midnight);
    let src = frame(w, h);
    let mut out = src.clone();
    draw_grade(&mut out, w, h, &p);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let want = apply_px(
                [
                    src[i + 2] as f32 / 255.0,
                    src[i + 1] as f32 / 255.0,
                    src[i] as f32 / 255.0,
                ],
                &p,
                (x as f32 + 0.5) / w as f32 - 0.5,
                (y as f32 + 0.5) / h as f32 - 0.5,
            );
            let got = [out[i + 2], out[i + 1], out[i]];
            for c in 0..3 {
                let w8 = (want[c] * 255.0).round() as u8;
                assert_eq!(got[c], w8, "pixel ({x}, {y}) channel {c}");
            }
        }
    }
}

#[test]
fn the_alpha_channel_is_never_touched() {
    let (w, h) = (8u32, 8u32);
    let mut out = frame(w, h);
    for i in (3..out.len()).step_by(4) {
        out[i] = 123;
    }
    draw_grade(&mut out, w, h, &params(GradePreset::Vivid));
    assert!(out.iter().skip(3).step_by(4).all(|&a| a == 123));
}

#[test]
fn noir_leaves_three_equal_channels_everywhere() {
    let (w, h) = (12u32, 9u32);
    let mut out = frame(w, h);
    draw_grade(&mut out, w, h, &params(GradePreset::Noir));
    for p in out.chunks(4) {
        assert!(
            p[0] == p[1] && p[1] == p[2],
            "saturation zero should be grey: {p:?}"
        );
    }
}
