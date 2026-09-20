use crate::export::fx::fx_masks::MaskDraw;
use crate::export::fx::fx_state::{FxRenderer, FxState};
use crate::export::fx::fxdraw::CpuFx;
use crate::export::grade::{apply_px, params_of, seed_of, GradeParams};
use crate::settings::grade::{GradePreset, GradeSettings};

const W: u32 = 64;
const H: u32 = 64;
const GREY: u8 = 128;
const DIM: f32 = 0.6;

fn noir() -> GradeParams {
    let (exposure, contrast, vignette) = seed_of(GradePreset::Noir);
    params_of(&GradeSettings {
        preset: GradePreset::Noir,
        exposure,
        contrast,
        vignette,
    })
    .unwrap()
}

fn at(src: &str, from: usize, needle: &str) -> usize {
    src[from..]
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} is gone from the seam"))
}

#[test]
fn the_grade_is_applied_to_the_masked_frame_not_the_other_way_round() {
    let st = FxState {
        masks: vec![MaskDraw {
            mn: [20.0, 20.0],
            mx: [44.0, 44.0],
            r: 0.0,
            feather_px: 1.0,
            amount_px: 0.0,
            dim: DIM,
            kind: 3,
            alpha: 1.0,
        }],
        grade: Some(noir()),
        ..Default::default()
    };
    let mut out = vec![GREY; (W * H * 4) as usize];
    CpuFx.apply(&mut out, W, H, &st);

    let (x, y) = (5u32, 5u32);
    let (u, v) = (
        (x as f32 + 0.5) / W as f32 - 0.5,
        (y as f32 + 0.5) / H as f32 - 0.5,
    );
    let p = noir();
    let keep = 1.0 - DIM;
    let byte = |c: [f32; 3]| (c[0] * 255.0).round();
    let masked = (GREY as f32 * keep).round();
    let grade_of_mask = byte(apply_px([masked / 255.0; 3], &p, u, v));
    let mask_of_grade = (byte(apply_px([GREY as f32 / 255.0; 3], &p, u, v)) * keep).round();

    let i = ((y * W + x) * 4) as usize;
    assert_eq!(
        out[i + 2] as f32,
        grade_of_mask,
        "outside the highlight the frame must be grade(mask(px))"
    );
    assert!(
        (grade_of_mask - mask_of_grade).abs() > 2.0,
        "the two orders must be far enough apart to be a real pin: \
         grade(mask(px)) = {grade_of_mask}, mask(grade(px)) = {mask_of_grade}"
    );
}

#[test]
fn the_shader_masks_then_grades_ahead_of_every_effect() {
    let src = include_str!("fx.wgsl");
    let fs = at(src, 0, "fn fs_main");
    let (mask, grade) = (at(src, fs, "mask_fx("), at(src, fs, "grade_fx("));
    let (video, spot) = (at(src, fs, "VF_NEBULA"), at(src, fs, "SP_VIGNETTE"));
    assert!(mask < grade, "fs_main runs mask_fx before grade_fx");
    assert!(
        grade < video && video < spot,
        "and both ahead of the video FX and the spotlight"
    );
}

#[test]
fn the_cpu_renderer_draws_in_the_same_order_as_the_shader() {
    let src = include_str!("fxdraw.rs");
    let a = at(src, 0, "fn apply");
    let steps = [
        "draw_masks",
        "draw_grade",
        "draw_video",
        "draw_spot",
        "draw_clicks",
        "draw_lens",
    ];
    let found: Vec<usize> = steps.iter().map(|s| at(src, a, s)).collect();
    assert!(
        found.windows(2).all(|p| p[0] < p[1]),
        "CpuFx::apply must call {steps:?} in that order, got offsets {found:?}"
    );
}
