use super::*;

const CURVES: [&str; 6] = [
    "keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)",
    "keys(0 0 0 0 0.45 0 b,1 1 -0.25 0 0 0 b)",
    "keys(0 0 0 0 0.15 -0.06 b,1 1 -0.4 0 0 0 b)",
    "keys(0 0 0 0 0 0 l,0.85 1 0 0 0 0 h,1 1 0 0 0 0 l)",
    "keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)",
    "keys(0 0 0 0 0.15 0.5 b,0.5 0.8 -0.1 0 0 0 h,0.7 0.8 0 0 0 0 l,0.85 1.15 0 0 0.05 0 b,1 1 -0.05 0 0 0 b)",
];

const CANON: [&str; 6] = [
    "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)",
    "keys(0.000 0.000 0.000 0.000 0.450 0.000 b,1.000 1.000 -0.250 0.000 0.000 0.000 b)",
    "keys(0.000 0.000 0.000 0.000 0.150 -0.060 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)",
    "keys(0.000 0.000 0.000 0.000 0.000 0.000 l,0.850 1.000 0.000 0.000 0.000 0.000 h,1.000 1.000 0.000 0.000 0.000 0.000 l)",
    "keys(0.000 0.000 0.000 0.000 0.333 0.000 b,1.000 1.000 -0.333 0.000 0.000 0.000 b)",
    "keys(0.000 0.000 0.000 0.000 0.150 0.500 b,0.500 0.800 -0.100 0.000 0.000 0.000 h,0.700 0.800 0.000 0.000 0.000 0.000 l,0.850 1.150 0.000 0.000 0.050 0.000 b,1.000 1.000 -0.050 0.000 0.000 0.000 b)",
];

const PS: [f32; 7] = [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];

const VALS: [[f32; 7]; 6] = [
    [
        0.000002, 0.363634, 0.617845, 0.847026, 0.962607, 0.993910, 1.000000,
    ],
    [
        0.000000, 0.016435, 0.101738, 0.388134, 0.771127, 0.953911, 1.000000,
    ],
    [
        0.000000, 0.054953, 0.251182, 0.606572, 0.884539, 0.979511, 1.000000,
    ],
    [
        0.000000, 0.117647, 0.294118, 0.588235, 0.882353, 1.000000, 1.000000,
    ],
    [
        0.000000, 0.028039, 0.156356, 0.499999, 0.843644, 0.971961, 1.000000,
    ],
    [
        0.000001, 0.275200, 0.559285, 0.800000, 0.916667, 1.111111, 1.000000,
    ],
];

#[test]
fn eval_is_pinned_at_fixed_instants() {
    for (c, want) in CURVES.iter().zip(VALS.iter()) {
        let k = parse_keys(c).unwrap_or_else(|| panic!("unparseable: {c}"));
        for (p, w) in PS.iter().zip(want.iter()) {
            let got = eval(&k, *p);
            assert!((got - w).abs() < 1e-4, "{c} at {p}: got {got}, want {w}");
        }
    }
}

#[test]
fn soft_matches_smoothstep() {
    let k = parse_keys(CURVES[4]).unwrap();
    for i in 0..=100 {
        let p = i as f32 / 100.0;
        let want = 3.0 * p * p - 2.0 * p * p * p;
        assert!(
            (eval(&k, p) - want).abs() < 1e-3,
            "at {p}: {} vs {want}",
            eval(&k, p)
        );
    }
}

#[test]
fn parsing_then_writing_gives_the_canonical_string() {
    for (c, want) in CURVES.iter().zip(CANON.iter()) {
        let k = parse_keys(c).unwrap();
        assert_eq!(format_keys(&k), *want);
        assert_eq!(format_keys(&parse_keys(want).unwrap()), *want);
    }
}

#[test]
fn handles_are_clamped_into_their_own_segment() {
    let k = parse_keys("keys(0 0 0 0 0.9 0.2 b,0.5 0.5 -0.9 0 0.9 0 b,1 1 -0.9 0 0 0 b)").unwrap();
    let ks = k.slice();
    assert_eq!(ks[0].out, [0.5, 0.2]);
    assert_eq!(ks[1].in_[0], -0.5);
    assert_eq!(ks[1].out[0], 0.5);
    assert_eq!(ks[2].in_[0], -0.5);
    assert_eq!(ks[0].in_, [0.0, 0.0]);
    assert_eq!(ks[2].out, [0.0, 0.0]);
}

#[test]
fn unsorted_keys_are_sorted_by_time() {
    let k = parse_keys("keys(1 1 -0.4 0 0 0 b,0 0 0 0 0.1 0.7 b)").unwrap();
    assert_eq!(format_keys(&k), CANON[0]);
}

#[test]
fn negative_zero_is_written_as_plain_zero() {
    let k = parse_keys("keys(0 0 0 0 0.5 0 b,0.5 0.5 -0.0001 0 0 0 b,1 1 -0.5 0 0 0 b)").unwrap();
    assert!(!format_keys(&k).contains("-0.000"));
}

#[test]
fn malformed_curves_are_unparseable() {
    let nine = "keys(0 0 0 0 0 0 l,0.125 0.1 0 0 0 0 l,0.25 0.2 0 0 0 0 l,0.375 0.3 0 0 0 0 l,\
        0.5 0.4 0 0 0 0 l,0.625 0.5 0 0 0 0 l,0.75 0.6 0 0 0 0 l,0.875 0.8 0 0 0 0 l,1 1 0 0 0 0 l)";
    for bad in [
        nine,
        "keys(0.1 0 0 0 0 0 l,1 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 l,0.9 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 z,1 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 l,1 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 0 l,1 1 0 0 0 0 l)",
        "keys(0 x 0 0 0 0 l,1 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 l,0.5 1 0 0 0 0 l,0.5 1 0 0 0 0 l,1 1 0 0 0 0 l)",
        "keys(0 0 0 0 0 0 l,1 1 0 0 0 0 l",
        "cubic(0.25,0.1,0.25,1)",
    ] {
        assert!(parse_keys(bad).is_none(), "should not parse: {bad}");
    }
}

#[test]
fn input_is_clamped_and_output_is_not() {
    let k = parse_keys(CURVES[5]).unwrap();
    assert!((eval(&k, -3.0) - eval(&k, 0.0)).abs() < 1e-6);
    assert!((eval(&k, 7.0) - eval(&k, 1.0)).abs() < 1e-6);
    assert!(
        eval(&k, 0.9) > 1.0,
        "the hand-made curve overshoots and stays overshot"
    );
}

#[test]
fn a_hold_segment_is_flat() {
    let k = parse_keys(CURVES[5]).unwrap();
    for p in [0.5f32, 0.55, 0.6, 0.65, 0.699] {
        assert!((eval(&k, p) - 0.8).abs() < 1e-6, "hold broke at {p}");
    }
}

#[test]
fn both_ease_entry_points_route_to_this_evaluator() {
    let k = parse_keys(CURVES[0]).unwrap();
    let e = crate::export::types::Easing::Keys(k);
    assert!((crate::export::easing::ease(e, 0.25) - VALS[0][2]).abs() < 1e-4);
    assert!((crate::export::camera::ease(e, 0.25) - VALS[0][2]).abs() < 1e-4);
}
