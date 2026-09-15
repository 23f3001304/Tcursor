use super::*;

fn sweep(k: f32, c: f32, m: f32) -> Vec<f32> {
    (0..=2000)
        .map(|i| spring(k, c, m, i as f32 / 2000.0))
        .collect()
}
fn peak(v: &[f32]) -> f32 {
    v.iter().fold(f32::MIN, |a, b| a.max(*b))
}

#[test]
fn endpoints_are_exact_for_every_branch() {
    for (k, c, m) in [
        (170.0, 26.0, 1.0),
        (300.0, 10.0, 1.0),
        (170.0, 60.0, 1.0),
        (170.0, 26.077, 1.0),
        (2000.0, 0.0, 1.0),
        (1.0, 200.0, 10.0),
    ] {
        assert_eq!(spring(k, c, m, 0.0), 0.0, "p=0 must be 0 for {k}/{c}/{m}");
        assert_eq!(spring(k, c, m, 1.0), 1.0, "p=1 must be 1 for {k}/{c}/{m}");
        assert!(
            (spring(k, c, m, 0.999) - 1.0).abs() < 1e-3,
            "{k}/{c}/{m} jumps at the end"
        );
        assert_eq!(spring(k, c, m, -0.5), 0.0);
        assert_eq!(spring(k, c, m, 2.0), 1.0);
        assert_eq!(spring(k, c, m, f32::NAN), 0.0);
    }
}

#[test]
fn underdamped_overshoots_and_more_so_the_lower_the_damping_ratio() {
    let (a, b, c) = (
        peak(&sweep(300.0, 10.0, 1.0)),
        peak(&sweep(100.0, 10.0, 1.0)),
        peak(&sweep(170.0, 26.0, 1.0)),
    );
    assert!(
        a > b && b > c,
        "overshoot must grow as damping falls: {a} {b} {c}"
    );
    assert!(
        (a - 1.388).abs() < 0.01 && (b - 1.163).abs() < 0.01,
        "peaks moved: {a} {b}"
    );
    assert!(c < 1.0 + 1e-4, "170/26 should not visibly overshoot: {c}");
}

#[test]
fn critical_and_overdamped_are_monotone() {
    for (k, c, m) in [(170.0, 26.077, 1.0), (170.0, 60.0, 1.0), (1.0, 200.0, 10.0)] {
        let v = sweep(k, c, m);
        for w in v.windows(2) {
            assert!(
                w[1] >= w[0] - 1e-6,
                "{k}/{c}/{m} went backwards: {} -> {}",
                w[0],
                w[1]
            );
        }
    }
}

#[test]
fn the_shape_depends_on_the_damping_ratio_alone() {
    for i in 1..10 {
        let p = i as f32 / 10.0;
        let (a, b) = (spring(170.0, 26.0, 1.0, p), spring(680.0, 52.0, 1.0, p));
        assert!(
            (a - b).abs() < 1e-4,
            "same zeta must be the same curve at p={p}: {a} vs {b}"
        );
    }
}

#[test]
fn params_are_clamped_and_zero_damping_still_settles() {
    let v = sweep(2000.0, 0.0, 1.0);
    assert!(
        v.iter().all(|x| x.is_finite()),
        "zero damping produced a non-finite sample"
    );
    assert!(
        peak(&v) > 1.5,
        "zero damping should ring hard: {}",
        peak(&v)
    );
    assert!(spring(1e9, -5.0, 1e9, 0.5).is_finite());
}

#[test]
fn wire_form_round_trips_and_defaults_mass() {
    assert_eq!(parse_spring("spring(170,26)"), Some((170.0, 26.0, 1.0)));
    assert_eq!(
        parse_spring("spring( 300 , 10 , 2 )"),
        Some((300.0, 10.0, 2.0))
    );
    assert_eq!(
        format_spring(170.0, 26.0, 1.0),
        "spring(170.000,26.000,1.000)"
    );
    let (k, c, m) = parse_spring(&format_spring(300.0, 10.0, 2.0)).unwrap();
    assert_eq!((k, c, m), (300.0, 10.0, 2.0));
    assert_eq!(
        parse_spring("spring(99999,-3,99)"),
        Some((2000.0, 0.0, 10.0))
    );
    for bad in [
        "spring(170)",
        "spring(1,2,3,4)",
        "spring()",
        "spring(a,b)",
        "spring(1,2",
        "cubic(0,0,1,1)",
    ] {
        assert_eq!(parse_spring(bad), None, "{bad} must not parse");
    }
}

#[test]
fn parity_table_matches_the_ts_mirror() {
    let table: [(f32, f32, f32, [f32; 9]); 4] = [
        (
            170.0,
            26.0,
            1.0,
            [
                0.154220, 0.405341, 0.618152, 0.768274, 0.865271, 0.924838, 0.960285, 0.980989,
                0.992996,
            ],
        ),
        (
            300.0,
            10.0,
            1.0,
            [
                1.216900, 1.107563, 0.874016, 1.055725, 0.994093, 0.989035, 1.007699, 0.996468,
                0.999043,
            ],
        ),
        (
            170.0,
            60.0,
            1.0,
            [
                0.471265, 0.735163, 0.867478, 0.933845, 0.967160, 0.983910, 0.992357, 0.996643,
                0.998844,
            ],
        ),
        (
            100.0,
            10.0,
            1.0,
            [
                0.547465, 1.085405, 1.145100, 1.031899, 0.975458, 0.983249, 1.000326, 1.004673,
                1.002020,
            ],
        ),
    ];
    for (k, c, m, want) in table {
        for (i, w) in want.iter().enumerate() {
            let p = (i + 1) as f32 / 10.0;
            let got = spring(k, c, m, p);
            assert!(
                (got - w).abs() < 1e-4,
                "spring({k},{c},{m}) at p={p}: {got} want {w}"
            );
        }
    }
}
