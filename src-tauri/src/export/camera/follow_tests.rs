use super::*;
use crate::export::camera::CameraSim;
use crate::export::types::{Easing, ZoomConfig};

const FW: f32 = 1920.0;

const STEP: f32 = REF_STEP_MS;
const FH: f32 = 1080.0;

fn region(follow: bool) -> ZoomRegion {
    ZoomRegion {
        start_ms: 0,
        end_ms: 4000,
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        target_scale: 2.2,
        anchor: FramePoint { x: 1500, y: 540 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: follow,
    }
}

#[test]
fn an_anchored_region_aims_at_its_anchor_wherever_the_cursor_is() {
    let r = region(false);
    for cx in [0, 400, 1400, 1500, 1600, 1919] {
        let c = FramePoint {
            x: cx,
            y: if cx % 2 == 0 { 20 } else { 1060 },
        };
        let (x, y) = aim(&r, c);
        assert!(
            (x - 1500.0).abs() < 1e-3 && (y - 540.0).abs() < 1e-3,
            "a Region zoom never follows the cursor (cursor at {cx}): aim {x},{y}"
        );
    }
}

#[test]
fn the_aim_is_its_own_fixed_point() {
    let r = region(false);
    for cursor_x in [0, 400, 900, 1500, 1919] {
        let c = FramePoint {
            x: cursor_x,
            y: 540,
        };
        let (x, y) = aim(&r, c);
        let parked = ZoomRegion {
            anchor: FramePoint {
                x: x as i32,
                y: y as i32,
            },
            ..r
        };
        let (x2, y2) = aim(&parked, c);
        assert!(
            (x2 - x).abs() < 1.0 && (y2 - y).abs() < 1.0,
            "aim moved when re-applied from its own output at cursor {cursor_x}: {x} -> {x2}"
        );
    }
}

#[test]
fn a_follow_region_aims_at_the_live_cursor_and_ignores_the_anchor() {
    let c = FramePoint { x: 300, y: 900 };
    let (x, y) = aim(&region(true), c);
    assert!(
        (x - 300.0).abs() < 1e-3 && (y - 900.0).abs() < 1e-3,
        "follow aim must be the cursor: {x},{y}"
    );
}

fn drive(r: &[ZoomRegion], t1: u32, cur: impl Fn(u32) -> FramePoint) -> (Vec<u32>, Vec<f32>) {
    let (cfg, mut s) = (ZoomConfig::default(), CameraSim::new(FW as u32, FH as u32));
    let (mut ts, mut xs) = (vec![], vec![]);
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > t1 {
            break;
        }
        let c = s.step(t, STEP, cur(t), r, &cfg);
        ts.push(t);
        xs.push(c.cx);
    }
    (ts, xs)
}

#[test]
fn an_anchored_ramp_lands_on_the_hold_pose_and_does_not_reverse() {
    let r = ZoomRegion {
        anchor: FramePoint { x: 674, y: 540 },
        ..region(false)
    };
    let (ts, xs) = drive(&[r], 900, |_| FramePoint { x: 1674, y: 540 });
    let (mut worst_dv, mut at) = (0.0f32, 0u32);
    for i in 2..xs.len() {
        let dv = (xs[i] - xs[i - 1]) - (xs[i - 1] - xs[i - 2]);
        if dv.abs() > worst_dv {
            worst_dv = dv.abs();
            at = ts[i];
        }
        assert!(
            (xs[i] - xs[i - 1]) <= 1e-3,
            "centre reversed at t={} ({} -> {})",
            ts[i],
            xs[i - 1],
            xs[i]
        );
    }
    assert!(
        worst_dv < 6.0,
        "ramp -> hold is still a jerk spike: {worst_dv:.2} px/frame^2 at t={at}"
    );
    let land = xs[xs.len() - 1];
    assert!(
        (land - 674.0).abs() < 1.0,
        "the camera settled off its anchor: {land}"
    );
}

#[test]
fn the_damping_factor_is_a_time_constant_not_a_per_step_one() {
    let tau = |k: f32, dt: f32| -dt / (1.0f32 - k).ln();
    for k in [0.02f32, 0.10, 0.36, 0.75] {
        assert!(
            (damping(k, REF_STEP_MS) - k).abs() < 1e-6,
            "k={k} must be itself at 60fps"
        );
        let t0 = tau(k, REF_STEP_MS);
        for dt in [1.0f32, 8.0, 16.0, 17.0, 33.0, 50.0] {
            let t = tau(damping(k, dt), dt);
            assert!(
                (t / t0 - 1.0).abs() < 1e-3,
                "k={k} dt={dt}: tau {t:.2}ms vs {t0:.2}ms"
            );
        }
    }
    let half = damping(0.10, REF_STEP_MS / 2.0);
    assert!(
        (1.0 - (1.0 - half) * (1.0 - half) - 0.10).abs() < 1e-6,
        "half-steps must compose"
    );
    assert_eq!(damping(0.0, 33.0), 0.0);
    assert_eq!(damping(1.0, 1.0), 1.0);
}

#[test]
fn the_export_grid_no_longer_ripples_the_damping_factor() {
    let cfg = ZoomConfig {
        follow_damping: 0.02,
        ..ZoomConfig::default()
    };
    let (r, aim_x) = ([region(true)], 500.0);
    let cur = |t: u32| FramePoint {
        x: if t < 400 { 1500 } else { 500 },
        y: 540,
    };
    let mut s = CameraSim::new(FW as u32, FH as u32);
    let (mut ts, mut xs) = (vec![], vec![]);
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > 1200 {
            break;
        }
        ts.push(t);
        xs.push(s.step(t, STEP, cur(t), &r, &cfg).cx);
    }
    let ks: Vec<f32> = (1..xs.len())
        .filter(|i| ts[*i] >= 450)
        .map(|i| (xs[i] - aim_x) / (xs[i - 1] - aim_x))
        .take(20)
        .collect();
    assert_eq!(
        ks.len(),
        20,
        "need 20 consecutive hold-phase frames, got {}",
        ks.len()
    );
    for (i, k) in ks.iter().enumerate() {
        assert!(
            (k - ks[0]).abs() < 1e-6,
            "the effective k rippled at frame {i}: {k} vs {}",
            ks[0]
        );
    }
    assert!(
        (ks[0] - 0.98).abs() < 1e-3,
        "1 - k should be the configured 0.98 at 60fps: {}",
        ks[0]
    );
    let spread = (1..=20u64)
        .map(|k| ((k * 1000 / 60) - ((k - 1) * 1000 / 60)) as f32)
        .map(|dt| damping(0.10, dt))
        .fold((1.0f32, 0.0f32), |(lo, hi), k| (lo.min(k), hi.max(k)));
    assert!(
        spread.1 - spread.0 > 0.005,
        "the rounded-dt control must ripple: {spread:?}"
    );
}

#[test]
fn the_zoom_out_ramp_keeps_following_instead_of_freezing_the_centre() {
    let cur = |t: u32| FramePoint {
        x: 300 + (t as f32 * 0.25) as i32,
        y: 540,
    };
    let r = [ZoomRegion {
        end_ms: 3000,
        ..region(true)
    }];
    let (ts, xs) = drive(&r, 2900, cur);
    let mean = |a: u32, b: u32| {
        let v: Vec<f32> = (1..xs.len())
            .filter(|i| (a..=b).contains(&ts[*i]))
            .map(|i| (xs[i] - xs[i - 1]).abs())
            .collect();
        v.iter().sum::<f32>() / v.len().max(1) as f32
    };
    let (before, after) = (mean(2400, 2540), mean(2560, 2700));
    assert!(
        after > before * 0.5,
        "the ramp-out still freezes the centre: {before:.2} -> {after:.2}"
    );
    let (cfg, mut s) = (ZoomConfig::default(), CameraSim::new(FW as u32, FH as u32));
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > 3000 {
            break;
        }
        s.step(t, STEP, cur(t), &r, &cfg);
    }
    let end = s.step(3000, STEP, cur(3000), &r, &cfg).scale;
    assert!(
        (end - 1.0).abs() < 1e-3,
        "scale must still reach 1 at end_ms: {end}"
    );
}

#[test]
fn a_follow_ramp_keeps_tracking_across_the_hold_boundary() {
    let cur = |t: u32| FramePoint {
        x: 300 + (t as f32 * 0.6) as i32,
        y: 540,
    };
    let (ts, xs) = drive(&[region(true)], 900, cur);
    let v: Vec<f32> = (1..xs.len()).map(|i| xs[i] - xs[i - 1]).collect();
    let mean = |a: u32, b: u32| {
        let s: Vec<f32> = (1..xs.len())
            .filter(|i| (a..=b).contains(&ts[*i]))
            .map(|i| v[i - 1].abs())
            .collect();
        s.iter().sum::<f32>() / s.len().max(1) as f32
    };
    let after = mean(500, 900);
    assert!(
        after > 5.0,
        "the follow stalled after the hold boundary: {after:.2} px/frame (cursor 10)"
    );
    assert!(
        after < 11.0,
        "the follow overshot the cursor's own speed: {after:.2} px/frame"
    );
}
