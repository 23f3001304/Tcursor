use crate::settings::grade::{GradePreset, GradeSettings};

pub mod gradedraw;

pub const LUMA_R: f32 = 0.2126;
pub const LUMA_G: f32 = 0.7152;
pub const LUMA_B: f32 = 0.0722;
pub const VIGN_IN: f32 = 0.45;
pub const CORNER: f32 = 0.70710678;
pub const TEMP_GAIN: f32 = 0.25;
pub const TINT_GAIN: f32 = 0.20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradeParams {
    pub exposure: f32,
    pub contrast: f32,
    pub vignette: f32,
    pub saturation: f32,
    pub temp: f32,
    pub tint: f32,
    pub lift: [f32; 3],
    pub gamma: [f32; 3],
    pub gain: [f32; 3],
}

type Row = (f32, f32, f32, f32, f32, f32, [f32; 3], [f32; 3], [f32; 3]);

fn row(p: GradePreset) -> Row {
    match p {
        GradePreset::None => (
            0.00, 1.00, 0.00, 1.00, 0.00, 0.00, [0.0; 3], [1.0; 3], [1.0; 3],
        ),
        GradePreset::Cinematic => (
            0.00,
            1.12,
            0.28,
            0.92,
            -0.08,
            0.02,
            [0.012, 0.016, 0.030],
            [1.00, 1.00, 1.04],
            [1.00, 0.99, 0.96],
        ),
        GradePreset::Noir => (
            0.05,
            1.30,
            0.40,
            0.00,
            0.00,
            0.00,
            [0.010, 0.010, 0.010],
            [0.96, 0.96, 0.96],
            [1.02, 1.02, 1.02],
        ),
        GradePreset::Vintage => (
            0.02,
            0.92,
            0.34,
            0.78,
            0.14,
            0.04,
            [0.045, 0.035, 0.020],
            [1.05, 1.02, 0.98],
            [0.97, 0.96, 0.92],
        ),
        GradePreset::Frost => (
            0.04,
            1.06,
            0.16,
            0.86,
            -0.22,
            -0.03,
            [0.010, 0.018, 0.030],
            [1.00, 1.02, 1.05],
            [0.98, 1.00, 1.03],
        ),
        GradePreset::Golden => (
            0.08,
            1.05,
            0.22,
            1.06,
            0.20,
            0.05,
            [0.020, 0.012, 0.000],
            [1.02, 1.00, 0.96],
            [1.04, 1.00, 0.93],
        ),
        GradePreset::Midnight => (
            -0.12,
            1.18,
            0.42,
            0.84,
            -0.16,
            -0.06,
            [0.000, 0.004, 0.030],
            [0.96, 0.98, 1.06],
            [0.94, 0.97, 1.05],
        ),
        GradePreset::Vivid => (
            0.04, 1.14, 0.10, 1.28, 0.02, 0.00, [0.0; 3], [1.0; 3], [1.0; 3],
        ),
        GradePreset::Dreamy => (
            0.10,
            0.88,
            0.18,
            1.10,
            0.06,
            0.03,
            [0.055, 0.050, 0.060],
            [1.06, 1.05, 1.06],
            [0.98, 0.98, 1.00],
        ),
    }
}

pub fn seed_of(p: GradePreset) -> (f32, f32, f32) {
    let r = row(p);
    (r.0, r.1, r.2)
}

pub fn params_of(s: &GradeSettings) -> Option<GradeParams> {
    if s.is_identity() {
        return None;
    }
    let r = row(s.preset);
    Some(GradeParams {
        exposure: s.exposure,
        contrast: s.contrast,
        vignette: s.vignette,
        saturation: r.3,
        temp: r.4,
        tint: r.5,
        lift: r.6,
        gamma: r.7,
        gain: r.8,
    })
}

pub fn vignette_k(u: f32, v: f32) -> f32 {
    let d = (u * u + v * v).sqrt() / CORNER;
    ((d - VIGN_IN) / (1.0 - VIGN_IN)).clamp(0.0, 1.0)
}

pub fn apply_px(c: [f32; 3], p: &GradeParams, u: f32, v: f32) -> [f32; 3] {
    let e = p.exposure.exp2();
    let mut o = [
        c[0] * e * (1.0 + TEMP_GAIN * p.temp),
        c[1] * e * (1.0 + TINT_GAIN * p.tint),
        c[2] * e * (1.0 - TEMP_GAIN * p.temp),
    ];
    for i in 0..3 {
        let x = (p.lift[i] + (p.gain[i] - p.lift[i]) * o[i]).clamp(0.0, 1.0);
        o[i] = x.powf(1.0 / p.gamma[i].max(0.001));
    }
    for x in o.iter_mut() {
        *x = (*x - 0.5) * p.contrast + 0.5;
    }
    let l = LUMA_R * o[0] + LUMA_G * o[1] + LUMA_B * o[2];
    for x in o.iter_mut() {
        *x = l + (*x - l) * p.saturation;
    }
    let k = vignette_k(u, v);
    let m = 1.0 - p.vignette * k * k;
    [
        (o[0] * m).clamp(0.0, 1.0),
        (o[1] * m).clamp(0.0, 1.0),
        (o[2] * m).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
#[path = "grade_tests.rs"]
mod tests;
