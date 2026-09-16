use super::{GradePreset, GradeSettings};
use crate::settings::model::Settings;

#[test]
fn the_default_is_the_identity() {
    let g = GradeSettings::default();
    assert_eq!(g.preset, GradePreset::None);
    assert_eq!((g.exposure, g.contrast, g.vignette), (0.0, 1.0, 0.0));
    assert!(g.is_identity());
    assert!(!GradeSettings { vignette: 0.1, ..g }.is_identity());
}

#[test]
fn a_config_written_before_the_field_loads_the_identity() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.grade, GradeSettings::default());
    let s2: Settings = serde_json::from_str(r#"{"grade":{"preset":"noir"}}"#).unwrap();
    assert_eq!(s2.grade.preset, GradePreset::Noir);
    assert_eq!(s2.grade.contrast, 1.0);
}

#[test]
fn every_preset_name_is_lowercase_on_the_wire() {
    for (p, s) in [
        (GradePreset::None, "\"none\""),
        (GradePreset::Cinematic, "\"cinematic\""),
        (GradePreset::Noir, "\"noir\""),
        (GradePreset::Vintage, "\"vintage\""),
        (GradePreset::Frost, "\"frost\""),
        (GradePreset::Golden, "\"golden\""),
        (GradePreset::Midnight, "\"midnight\""),
        (GradePreset::Vivid, "\"vivid\""),
        (GradePreset::Dreamy, "\"dreamy\""),
    ] {
        assert_eq!(serde_json::to_string(&p).unwrap(), s);
        assert_eq!(serde_json::from_str::<GradePreset>(s).unwrap(), p);
    }
}

#[test]
fn a_chosen_grade_round_trips_through_settings() {
    let mut s = Settings::default();
    s.grade = GradeSettings {
        preset: GradePreset::Cinematic,
        exposure: 0.0,
        contrast: 1.12,
        vignette: 0.28,
    };
    let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.grade, s.grade);
}
