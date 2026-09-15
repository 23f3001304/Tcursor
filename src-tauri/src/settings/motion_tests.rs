use super::MotionSettings;
use crate::settings::model::Settings;

#[test]
fn default_is_soft_and_soft_is_the_bare_smooth_word() {
    let m = MotionSettings::default();
    assert_eq!(m.preset, "soft");
    assert_eq!(m.easing, "smooth");
    assert_eq!(m.easing_out, "smooth");
}

#[test]
fn a_config_written_before_the_field_loads_soft() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.motion, MotionSettings::default());
    let s2: Settings = serde_json::from_str(r#"{"ai_model":"llama3.2"}"#).unwrap();
    assert_eq!(s2.motion.preset, "soft");
}

#[test]
fn a_chosen_preset_round_trips_through_json() {
    let mut s = Settings::default();
    s.motion = MotionSettings {
        preset: "bouncy".into(),
        easing: "spring(140.000,7.000,1.000)".into(),
        easing_out: "spring(140.000,7.000,1.000)".into(),
    };
    let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert_eq!(back.motion, s.motion);
}

#[test]
fn partial_motion_json_fills_the_missing_halves_with_soft() {
    let m: MotionSettings = serde_json::from_str(r#"{"preset":"cinematic"}"#).unwrap();
    assert_eq!(
        (m.preset.as_str(), m.easing.as_str(), m.easing_out.as_str()),
        ("cinematic", "smooth", "smooth")
    );
}
