use super::*;

#[test]
fn the_four_styles_are_the_table_the_spec_fixes() {
    let clean = style_of("clean");
    assert!(clean.shadow && !clean.plate && !clean.rule);
    assert_eq!(fill_rgb(clean.fill, [239, 68, 68]), [255, 255, 255]);

    let plate = style_of("plate");
    assert!(plate.plate && !plate.shadow && !plate.rule);
    assert_eq!(plate.plate_rgb, [0, 0, 0]);
    assert!((plate.plate_alpha - 0.62).abs() < 1e-6);

    let accent = style_of("accent");
    assert!(accent.shadow && !accent.plate && !accent.rule);
    assert_eq!(fill_rgb(accent.fill, [239, 68, 68]), [239, 68, 68]);

    let bar = style_of("bar");
    assert!(bar.rule && !bar.shadow && !bar.plate);
    assert_eq!(fill_rgb(bar.fill, [239, 68, 68]), [255, 255, 255]);
}

#[test]
fn an_unknown_style_is_clean_rather_than_nothing() {
    assert_eq!(style_of("wobble"), style_of("clean"));
    assert_eq!(style_of(""), style_of("clean"));
}

#[test]
fn every_name_the_ops_module_accepts_has_a_row_here() {
    for name in crate::edit::ops::textops::TEXT_STYLES {
        assert_ne!(
            (style_of(name), name),
            (style_of("clean"), "plate"),
            "placeholder to force a per-name check"
        );
    }
    assert_eq!(crate::edit::ops::textops::TEXT_STYLES.len(), 4);
    assert!(style_of("plate").plate && style_of("bar").rule);
}
