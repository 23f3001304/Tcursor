use super::*;

#[test]
fn parses_well_formed_and_whitespaced_forms() {
    assert_eq!(
        parse_cubic("cubic(0.25,0.1,0.25,1)"),
        Some((0.25, 0.1, 0.25, 1.0))
    );
    assert_eq!(
        parse_cubic("  cubic( 0.42 , 0 , 0.58 , 1 ) "),
        Some((0.42, 0.0, 0.58, 1.0))
    );
}

#[test]
fn rejects_garbage() {
    for s in [
        "smooth",
        "cubic(",
        "cubic()",
        "cubic(1,2,3)",
        "cubic(1,2,3,4,5)",
        "cubic(a,b,c,d)",
        "cubic(0.1,0.2,0.3,)",
        "bezier(0,0,1,1)",
        "",
    ] {
        assert!(parse_cubic(s).is_none(), "{s:?} must not parse as a cubic");
    }
    assert!(
        parse_cubic("cubic(0,0,inf,1)").is_none(),
        "non-finite coords must not parse"
    );
    assert!(
        parse_cubic("cubic(0,NaN,1,1)").is_none(),
        "NaN coords must not parse"
    );
}

#[test]
fn clamps_x_into_0_1_but_leaves_y_free_to_overshoot() {
    assert_eq!(parse_cubic("cubic(-3,-2,9,4)"), Some((0.0, -2.0, 1.0, 4.0)));
}

#[test]
fn format_round_trips_and_is_byte_stable() {
    let s = format_cubic(0.25, 0.1, 0.25, 1.0);
    assert_eq!(s, "cubic(0.250,0.100,0.250,1.000)");
    let (a, b, c, d) = parse_cubic(&s).expect("canonical form must re-parse");
    assert_eq!(
        format_cubic(a, b, c, d),
        s,
        "re-formatting must be idempotent"
    );
}

#[test]
fn endpoints_are_exact() {
    for (x1, y1, x2, y2) in [
        (0.25, 0.1, 0.25, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.34, 1.56, 0.64, 1.0),
    ] {
        assert_eq!(eval(x1, y1, x2, y2, 0.0), 0.0);
        assert_eq!(eval(x1, y1, x2, y2, 1.0), 1.0);
        assert_eq!(eval(x1, y1, x2, y2, -5.0), 0.0, "input is clamped");
        assert_eq!(eval(x1, y1, x2, y2, 5.0), 1.0, "input is clamped");
    }
}

#[test]
fn matches_the_css_ease_curve() {
    assert!((eval(0.25, 0.1, 0.25, 1.0, 0.5) - 0.802_403).abs() < 1e-3);
    assert!((eval(0.25, 0.1, 0.25, 1.0, 0.25) - 0.408_511).abs() < 1e-3);
    assert!((eval(0.42, 0.0, 0.58, 1.0, 0.5) - 0.5).abs() < 1e-4);
    for p in [0.1, 0.3, 0.5, 0.9] {
        assert!((eval(1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, p) - p).abs() < 1e-3);
    }
}

#[test]
fn is_monotonic_in_x_for_in_range_handles() {
    for (x1, x2) in [(0.0, 1.0), (1.0, 0.0), (0.0, 0.0), (1.0, 1.0), (0.25, 0.25)] {
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = eval(x1, 0.0, x2, 1.0, i as f32 / 100.0);
            assert!(
                v >= prev - 1e-5,
                "eval({x1},0,{x2},1) dipped at p={}",
                i as f32 / 100.0
            );
            prev = v;
        }
    }
}

#[test]
fn overshoots_past_one_when_a_handle_does() {
    assert!(eval(0.34, 1.56, 0.64, 1.0, 0.7) > 1.0);
    assert_eq!(eval(0.34, 1.56, 0.64, 1.0, 1.0), 1.0);
}
