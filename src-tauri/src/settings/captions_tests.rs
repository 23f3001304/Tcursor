use crate::settings::captions::{CaptionPos, CaptionSize, CaptionStyle};
use crate::settings::model::Settings;

#[test]
fn defaults_are_bottom_medium_pill_and_base_en() {
    let s = CaptionStyle::default();
    assert!(s.enabled);
    assert_eq!(s.position, CaptionPos::Bottom);
    assert_eq!(s.size, CaptionSize::M);
    assert!(s.pill);
    assert!(s.highlight);
    assert_eq!(s.model, "base.en");
    assert_eq!(s.language, "en");
}

#[test]
fn size_rungs_are_fractions_of_output_height() {
    assert!((CaptionSize::S.height_frac() - 0.030).abs() < 1e-6);
    assert!((CaptionSize::M.height_frac() - 0.038).abs() < 1e-6);
    assert!((CaptionSize::L.height_frac() - 0.048).abs() < 1e-6);
}

#[test]
fn a_settings_blob_written_before_captions_existed_loads_with_the_default_style() {
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.captions, CaptionStyle::default());
    assert!(!s.clickfx.captions);
}
