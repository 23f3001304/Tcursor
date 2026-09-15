use super::*;

const STEP60: f32 = 1000.0 / 60.0;
const STEP30: f32 = 1000.0 / 30.0;

fn throw(dt: f32, speed: f64, move_ms: f64, end_ms: f64, max: f32) -> Vec<(f64, f32)> {
    let mut f = Tilt::new();
    let n = (end_ms / dt as f64).round() as usize;
    (0..=n)
        .map(|i| {
            let t = i as f64 * dt as f64;
            let x = (speed * t.min(move_ms)) as f32;
            (t, f.step(x, 0.0, dt, max))
        })
        .collect()
}

fn at(run: &[(f64, f32)], t: f64) -> f32 {
    run.iter()
        .min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs()))
        .unwrap()
        .1
}

#[test]
fn a_fast_throw_leans_and_is_capped_by_the_setting() {
    let r = throw(STEP60, 2.0, 600.0, 600.0, MAX_DEG);
    let settled = at(&r, 500.0);
    assert!(settled > 1.0, "a 2 px/ms sweep must lean, got {settled}");
    for (tilt, speed) in [(1.0, 8.0), (0.35, 8.0), (0.35, 2.0)] {
        let max = max_deg(tilt);
        for (t, a) in throw(STEP60, speed, 600.0, 900.0, max) {
            assert!(
                a <= max * 1.2,
                "tilt {tilt} speed {speed}: {a} deg at {t} ms passes {max}"
            );
        }
        let held = throw(STEP60, speed, 1500.0, 1500.0, max).last().unwrap().1;
        assert!(
            (held - max).abs() < 0.01,
            "a sweep held at speed settles ON the cap: {held} vs {max}"
        );
    }
    let gentle = throw(STEP60, 8.0, 600.0, 600.0, max_deg(0.35));
    assert!(at(&gentle, 500.0) < at(&throw(STEP60, 8.0, 600.0, 600.0, MAX_DEG), 500.0));
}

#[test]
fn stopping_returns_to_upright_through_exactly_one_overshoot() {
    let r = throw(STEP60, 2.5, 400.0, 1200.0, MAX_DEG);
    let after: Vec<(f64, f32)> = r.iter().copied().filter(|(t, _)| *t >= 400.0).collect();
    let lows: Vec<(f64, f32)> = (1..after.len() - 1)
        .filter(|&i| {
            after[i].1 < -0.2 && after[i].1 <= after[i - 1].1 && after[i].1 <= after[i + 1].1
        })
        .map(|i| after[i])
        .collect();
    assert_eq!(lows.len(), 1, "one visible overshoot, got {lows:?}");
    let (t_peak, deg) = lows[0];
    assert!(
        deg > -1.5,
        "the overshoot must stay under 1.5 deg, got {deg}"
    );
    assert!(
        (150.0..300.0).contains(&(t_peak - 400.0)),
        "peak at {} ms after the stop",
        t_peak - 400.0
    );
    assert!(at(&r, 1000.0).abs() < 0.05, "settled: {}", at(&r, 1000.0));
}

#[test]
fn ordinary_pointing_stays_upright() {
    for speed in [0.05, 0.2, 0.39] {
        for (t, a) in throw(STEP60, speed, 2000.0, 2000.0, MAX_DEG) {
            assert!(a.abs() < 1e-6, "{speed} px/ms leaned {a} deg at {t} ms");
        }
    }
    let just_past = at(&throw(STEP60, 0.45, 600.0, 600.0, MAX_DEG), 500.0);
    assert!(
        (0.0..0.5).contains(&just_past),
        "just past the dead zone: {just_past}"
    );
}

#[test]
fn a_vertical_throw_leans_at_half_weight() {
    let mut down = Tilt::new();
    let mut right = Tilt::new();
    for i in 0..=36 {
        let (t, d) = (i as f64 * STEP60 as f64, STEP60);
        down.step(0.0, (2.0 * t) as f32, d, MAX_DEG);
        right.step((2.0 * t) as f32, 0.0, d, MAX_DEG);
    }
    let (dv, rv) = (down.angle_deg(), right.angle_deg());
    assert!(dv > 0.0 && rv > 0.0, "down {dv} right {rv}");
    assert!(
        (dv / rv - 0.5).abs() < 0.02,
        "vertical must lean at half weight: {dv} vs {rv}"
    );
}

#[test]
fn the_angle_is_the_same_at_30fps_as_at_60fps() {
    let (a, b) = (
        throw(STEP60, 2.5, 400.0, 1200.0, MAX_DEG),
        throw(STEP30, 2.5, 400.0, 1200.0, MAX_DEG),
    );
    for k in 1..=12 {
        let t = k as f64 * 100.0;
        assert!(
            (at(&a, t) - at(&b, t)).abs() < 1e-3,
            "t {t}: 60fps {} vs 30fps {}",
            at(&a, t),
            at(&b, t)
        );
    }
}

#[test]
fn tilt_zero_never_leans() {
    for (t, a) in throw(STEP60, 8.0, 600.0, 1200.0, max_deg(0.0)) {
        assert!(a.abs() < 1e-6, "tilt 0 leaned {a} deg at {t} ms");
    }
    assert_eq!(max_deg(0.0), 0.0);
    assert_eq!(max_deg(1.0), MAX_DEG);
    assert_eq!(max_deg(2.0), MAX_DEG, "the setting is clamped, not trusted");
}

#[test]
fn the_five_pins_the_typescript_mirror_must_match() {
    let r = throw(STEP60, 2.0, 200.0, 600.0, MAX_DEG);
    let pins = [
        (100.0, 1.213484),
        (200.0, 3.291586),
        (300.0, 1.707726),
        (400.0, -0.444513),
        (600.0, 0.070651),
    ];
    for (t, want) in pins {
        assert!(
            (at(&r, t) as f64 - want).abs() < 1e-3,
            "t {t}: {} want {want}",
            at(&r, t)
        );
    }
}

#[test]
fn a_frame_with_no_elapsed_time_only_records_the_position() {
    let mut f = Tilt::new();
    f.step(0.0, 0.0, STEP60, MAX_DEG);
    f.step(200.0, 0.0, 0.0, MAX_DEG);
    assert_eq!(f.angle_deg(), 0.0);
    assert_eq!(f.step(200.0, 0.0, STEP60, MAX_DEG), 0.0);
}

#[test]
fn the_reference_scale_makes_one_gesture_lean_the_same_at_any_capture_size() {
    assert!((ref_scale(1920) - 1.0).abs() < 1e-6);
    assert!((ref_scale(3840) - 0.5).abs() < 1e-6);
    assert!(
        ref_scale(0).is_finite(),
        "a degenerate screen width must not divide by zero"
    );
}
