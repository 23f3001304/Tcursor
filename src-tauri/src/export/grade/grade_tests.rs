use super::*;
use crate::settings::grade::{GradePreset, GradeSettings};

pub fn settings(p: GradePreset) -> GradeSettings {
    let (exposure, contrast, vignette) = seed_of(p);
    GradeSettings {
        preset: p,
        exposure,
        contrast,
        vignette,
    }
}

#[test]
fn the_identity_grade_resolves_to_no_parameters_at_all() {
    assert!(params_of(&GradeSettings::default()).is_none());
    assert!(params_of(&settings(GradePreset::None)).is_none());
    let mut bent = GradeSettings::default();
    bent.exposure = 0.05;
    assert!(
        params_of(&bent).is_some(),
        "a bent knob on the None look is still a grade"
    );
}

#[test]
fn the_identity_parameters_are_a_byte_identical_no_op_over_a_noise_frame() {
    let p = GradeParams {
        exposure: 0.0,
        contrast: 1.0,
        vignette: 0.0,
        saturation: 1.0,
        temp: 0.0,
        tint: 0.0,
        lift: [0.0; 3],
        gamma: [1.0; 3],
        gain: [1.0; 3],
    };
    for k in 0..=255u32 {
        let c = [k as u8, (255 - k) as u8, ((k * 7) % 256) as u8];
        let f = apply_px(
            [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
            ],
            &p,
            0.3,
            -0.2,
        );
        let back = [
            (f[0] * 255.0).round() as u8,
            (f[1] * 255.0).round() as u8,
            (f[2] * 255.0).round() as u8,
        ];
        assert_eq!(back, c, "the identity changed {c:?}");
    }
}

#[test]
fn picking_a_preset_seeds_the_three_stored_numbers_from_its_row() {
    assert_eq!(seed_of(GradePreset::None), (0.0, 1.0, 0.0));
    assert_eq!(seed_of(GradePreset::Cinematic), (0.0, 1.12, 0.28));
    assert_eq!(seed_of(GradePreset::Midnight), (-0.12, 1.18, 0.42));
    assert_eq!(seed_of(GradePreset::Dreamy), (0.10, 0.88, 0.18));
}

#[test]
fn the_stored_numbers_win_over_the_preset_row() {
    let mut s = settings(GradePreset::Noir);
    s.exposure = -1.0;
    s.contrast = 0.5;
    s.vignette = 0.0;
    let p = params_of(&s).unwrap();
    assert_eq!((p.exposure, p.contrast, p.vignette), (-1.0, 0.5, 0.0));
    assert_eq!(
        p.saturation, 0.0,
        "Noir's other eight parameters are still Noir's"
    );
}

#[path = "grade_table_tests.rs"]
mod table_tests;
