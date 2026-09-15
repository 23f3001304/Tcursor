use super::*;
use crate::export::types::{Aspect, Layout};

#[test]
fn default_export_settings_matches_todays_export() {
    let s = ExportSettings::default();
    assert_eq!(s.resolution, Resolution::Source);
    assert_eq!(s.fps, Fps::F60);
    assert_eq!(s.quality_crf, DEFAULT_CRF);
    assert_eq!(s.format, Format::Mp4);
}

#[test]
fn fps_f60_is_always_60_regardless_of_capture_rate() {
    assert_eq!(Fps::F60.resolve_hz(30), 60);
    assert_eq!(Fps::F60.resolve_hz(144), 60);
    assert_eq!(Fps::F60.resolve_hz(0), 60);
}

#[test]
fn fps_source_reproduces_the_old_fallback_formula() {
    assert_eq!(Fps::Source.resolve_hz(75), 75);
    assert_eq!(Fps::Source.resolve_hz(0), 60);
}

#[test]
fn fps_f30_is_always_30() {
    assert_eq!(Fps::F30.resolve_hz(60), 30);
}

#[test]
fn resolution_source_is_a_noop_on_layout() {
    let mut l = Layout::default();
    l.apply_aspect(Aspect::Wide16x9, 640, 480);
    let (w, h) = (l.out_w, l.out_h);
    l.rescale_to_resolution(Resolution::Source);
    assert_eq!((l.out_w, l.out_h), (w, h));
}

#[test]
fn resolution_presets_use_the_short_edge_convention() {
    let mut l = Layout::default();
    l.apply_aspect(Aspect::Wide16x9, 1, 1);
    l.rescale_to_resolution(Resolution::P720);
    assert_eq!((l.out_w, l.out_h), (1280, 720));

    let mut l = Layout::default();
    l.apply_aspect(Aspect::Wide16x9, 1, 1);
    l.rescale_to_resolution(Resolution::P2160);
    assert_eq!((l.out_w, l.out_h), (3840, 2160));

    let mut l = Layout::default();
    l.apply_aspect(Aspect::Vertical9x16, 1, 1);
    l.rescale_to_resolution(Resolution::P720);
    assert_eq!((l.out_w, l.out_h), (720, 1280));
}

#[test]
fn resolution_p1080_matches_every_fixed_aspect_preset_exactly() {
    for (aspect, want) in [
        (Aspect::Wide16x9, (1920u32, 1080u32)),
        (Aspect::Vertical9x16, (1080, 1920)),
        (Aspect::Square1x1, (1080, 1080)),
        (Aspect::Classic4x3, (1440, 1080)),
    ] {
        let mut l = Layout::default();
        l.apply_aspect(aspect, 1, 1);
        l.rescale_to_resolution(Resolution::P1080);
        assert_eq!((l.out_w, l.out_h), want, "{aspect:?}");
    }
}

#[test]
fn format_extensions_and_audio_support() {
    assert_eq!(Format::Mp4.extension(), "mp4");
    assert_eq!(Format::WebM.extension(), "webm");
    assert_eq!(Format::Gif.extension(), "gif");
    assert!(Format::Mp4.supports_audio());
    assert!(Format::WebM.supports_audio());
    assert!(!Format::Gif.supports_audio());
}

#[test]
fn serde_defaults_fill_in_missing_fields() {
    let s: ExportSettings = serde_json::from_str("{}").expect("empty object deserializes");
    assert_eq!(s, ExportSettings::default());
}
