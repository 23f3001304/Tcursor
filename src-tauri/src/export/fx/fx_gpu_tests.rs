use super::*;
use crate::export::fx::fx_state::{FxHit, Spot};
use crate::export::fx::fxdraw::CpuFx;
use crate::settings::model::{ClickFxStyle, SpotlightMode};

fn both(st: &FxState, w: u32, h: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    let Some(g) = GpuFx::new(w, h) else {
        eprintln!("fx_gpu_tests: SKIPPED - no wgpu adapter on this machine");
        return None;
    };
    let (mut a, mut b) = (
        vec![90u8; (w * h * 4) as usize],
        vec![90u8; (w * h * 4) as usize],
    );
    g.apply(&mut a, w, h, st);
    CpuFx.apply(&mut b, w, h, st);
    Some((a, b))
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
        style: ClickFxStyle::None,
        color: [0, 0, 0],
        intensity: 1.0,
        hits: vec![],
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
        video: None,
        lens: None,
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
                intensity: 1.0,
                hits: vec![FxHit {
                    x: 64.0,
                    y: 64.0,
                    progress,
                }],
                spot: None,
                video: None,
                lens: None,
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
            color: [0, 0, 0],
            intensity: 1.0,
            hits: vec![FxHit {
                x: 64.0,
                y: 64.0,
                progress: p,
            }],
            spot: None,
            video: None,
            lens: None,
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
        style: ClickFxStyle::None,
        color: [0, 0, 0],
        intensity: 1.0,
        hits: vec![],
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
        video: None,
        lens: None,
    };
    g.apply(&mut out, w, h, &st);
    assert!(
        out[0] < out[((32 * w + 32) * 4) as usize],
        "GPU spotlight: corner dimmer than center"
    );
}
