use super::*;
use crate::export::fx::fx_masks::MaskDraw;
use crate::export::fx::fx_state::{FxHit, Spot};
use crate::export::fx::fxdraw::CpuFx;
use crate::export::grade::{params_of, seed_of, GradeParams};
use crate::settings::grade::{GradePreset, GradeSettings};
use crate::settings::model::{ClickFxStyle, SpotlightMode};

fn both_on(base: &[u8], st: &FxState, w: u32, h: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    let Some(g) = GpuFx::new(w, h) else {
        eprintln!("fx_gpu_tests: SKIPPED - no wgpu adapter on this machine");
        return None;
    };
    let (mut a, mut b) = (base.to_vec(), base.to_vec());
    g.apply(&mut a, w, h, st);
    CpuFx.apply(&mut b, w, h, st);
    Some((a, b))
}

fn both(st: &FxState, w: u32, h: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    both_on(&vec![90u8; (w * h * 4) as usize], st, w, h)
}

fn added(base: u8, buf: &[u8]) -> i64 {
    buf.chunks(4)
        .map(|p| {
            (0..3)
                .map(|c| (p[c] as i64 - base as i64).max(0))
                .sum::<i64>()
        })
        .sum()
}

#[test]
fn cpu_spotlight_dim_matches_the_shader_at_probe_points() {
    let (w, h) = (128u32, 128u32);
    let st = FxState {
        spot: Some(Spot {
            cx: 64.0,
            cy: 64.0,
            dim: 0.6,
            radius_frac: 0.13,
            feather_frac: 0.10,
            alpha: 1.0,
            mode: SpotlightMode::Classic,
            tint: [0, 0, 0],
            t: 0.0,
            cam_rect: [0.0; 4],
            cam_radius: 0.0,
            dim_camera: true,
        }),
        ..Default::default()
    };
    let Some((gpu, cpu)) = both(&st, w, h) else {
        return;
    };
    for (px, py) in [(64u32, 64u32), (64, 76), (64, 90), (0, 0), (127, 127)] {
        let i = ((py * w + px) * 4) as usize;
        assert!(
            (gpu[i] as i32 - cpu[i] as i32).abs() <= 3,
            "classic dim at ({px},{py}): gpu {} vs cpu {}",
            gpu[i],
            cpu[i]
        );
    }
}

#[test]
fn cpu_click_ring_gains_match_the_shader() {
    let (w, h) = (128u32, 128u32);
    for style in [
        ClickFxStyle::Neon,
        ClickFxStyle::Shockwave,
        ClickFxStyle::Ripple,
        ClickFxStyle::Glow,
        ClickFxStyle::Pulse,
    ] {
        for progress in [0.05f32, 0.5, 0.8] {
            let st = FxState {
                style,
                color: [0, 128, 255],
                hits: vec![FxHit {
                    x: 64.0,
                    y: 64.0,
                    progress,
                }],
                ..Default::default()
            };
            let Some((gpu, cpu)) = both(&st, w, h) else {
                return;
            };
            let (ag, ac) = (added(90, &gpu), added(90, &cpu));
            assert!(
                ag > 0,
                "{style:?}@{progress}: the shader must actually draw something"
            );
            assert!(
                (ag - ac).abs() * 10 <= ag * 2,
                "{style:?}@{progress} added light: gpu {ag} vs cpu {ac} (>20% apart)"
            );
        }
    }
}

#[test]
fn the_impact_flash_is_brightest_at_the_click_and_gone_by_p_0_2() {
    let (w, h) = (128u32, 128u32);
    let at = |p: f32| {
        let st = FxState {
            style: ClickFxStyle::Particles,
            hits: vec![FxHit {
                x: 64.0,
                y: 64.0,
                progress: p,
            }],
            ..Default::default()
        };
        both(&st, w, h).map(|(gpu, _)| gpu[((64 * w + 64) * 4) as usize] as i32)
    };
    let (Some(p0), Some(p10), Some(p20)) = (at(0.0), at(0.10), at(0.20)) else {
        return;
    };
    assert!(
        p0 > 200,
        "flash blows the click point out at p=0 (got {p0} of 255)"
    );
    assert!(p10 < p0, "flash is already fading by p=0.10 ({p10} < {p0})");
    assert!(
        p20 <= 91,
        "flash is gone by p=0.20 - the base frame (90) is back, got {p20}"
    );
}

#[test]
fn spotlight_dims_corner_more_than_center() {
    let g = match GpuFx::new(64, 64) {
        Some(g) => g,
        None => return,
    };
    let (w, h) = (64u32, 64u32);
    let mut out = vec![200u8; (w * h * 4) as usize];
    let st = FxState {
        spot: Some(Spot {
            cx: 32.0,
            cy: 32.0,
            dim: 0.7,
            radius_frac: 0.13,
            feather_frac: 0.10,
            alpha: 1.0,
            mode: SpotlightMode::Classic,
            tint: [0, 0, 0],
            t: 0.0,
            cam_rect: [0.0; 4],
            cam_radius: 0.0,
            dim_camera: true,
        }),
        ..Default::default()
    };
    g.apply(&mut out, w, h, &st);
    assert!(
        out[0] < out[((32 * w + 32) * 4) as usize],
        "GPU spotlight: corner dimmer than center"
    );
}

fn ramp(w: u32, h: u32) -> Vec<u8> {
    let mut v = vec![255u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            v[i] = (x * 2) as u8;
            v[i + 1] = (y * 2) as u8;
            v[i + 2] = (x + y) as u8;
        }
    }
    v
}

fn seeded(p: GradePreset) -> GradeParams {
    let (exposure, contrast, vignette) = seed_of(p);
    params_of(&GradeSettings {
        preset: p,
        exposure,
        contrast,
        vignette,
    })
    .unwrap()
}

fn centred(kind: u32, amount_px: f32) -> MaskDraw {
    MaskDraw {
        mn: [32.0, 32.0],
        mx: [96.0, 96.0],
        r: 0.0,
        feather_px: 1.0,
        amount_px,
        dim: 0.6,
        kind,
        alpha: 1.0,
    }
}

#[test]
fn the_mask_and_grade_stages_agree_between_the_gpu_and_the_cpu() {
    let (w, h) = (128u32, 128u32);
    for (kind, amount_px, preset) in [
        (3u32, 0.0f32, GradePreset::Noir),
        (2, 9.0, GradePreset::Cinematic),
    ] {
        let st = FxState {
            masks: vec![centred(kind, amount_px)],
            grade: Some(seeded(preset)),
            ..Default::default()
        };
        let Some((gpu, cpu)) = both_on(&ramp(w, h), &st, w, h) else {
            return;
        };
        let (mut worst, mut at) = (0i32, 0usize);
        for i in (0..gpu.len()).filter(|i| i % 4 != 3) {
            let d = (gpu[i] as i32 - cpu[i] as i32).abs();
            if d > worst {
                (worst, at) = (d, i);
            }
        }
        assert!(
            worst <= 3,
            "mask kind {kind} under {preset:?}: {worst} of 255 apart at ({}, {}), gpu {} vs cpu {}",
            (at / 4) as u32 % w,
            (at / 4) as u32 / w,
            gpu[at],
            cpu[at]
        );
    }
}
