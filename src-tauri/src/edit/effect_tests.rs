use crate::edit::effect::{
    EffectKind, EffectRegion, DEFAULT_BLUR, DEFAULT_MASK_FEATHER, DEFAULT_MASK_ROUNDNESS,
    DEFAULT_PIXEL,
};
use crate::edit::model::EditDoc;

fn spot() -> EffectRegion {
    EffectRegion {
        id: "e0".into(),
        kind: EffectKind::Spotlight,
        start_ms: 100,
        end_ms: 900,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
        rect: None,
        strength: None,
        roundness: None,
    }
}

#[test]
fn a_region_written_before_masks_existed_loads_with_no_rect() {
    let json = r#"{"id":"e0","kind":"spotlight","start_ms":0,"end_ms":900,"layer":0}"#;
    let e: EffectRegion = serde_json::from_str(json).unwrap();
    assert_eq!(e.kind, EffectKind::Spotlight);
    assert_eq!((e.rect, e.strength, e.roundness), (None, None, None));
    assert_eq!((e.fade_in_ms, e.fade_out_ms), (250, 250));
}

#[test]
fn the_three_mask_kinds_serialize_lowercase_and_spotlight_is_not_a_mask() {
    for (k, s) in [
        (EffectKind::Blur, "\"blur\""),
        (EffectKind::Pixelate, "\"pixelate\""),
        (EffectKind::Highlight, "\"highlight\""),
        (EffectKind::Spotlight, "\"spotlight\""),
    ] {
        assert_eq!(serde_json::to_string(&k).unwrap(), s);
        assert_eq!(serde_json::from_str::<EffectKind>(s).unwrap(), k);
        assert_eq!(k.is_mask(), !matches!(k, EffectKind::Spotlight));
    }
}

#[test]
fn a_mask_round_trips_its_rect_strength_and_roundness_through_a_doc() {
    let mut e = spot();
    e.kind = EffectKind::Blur;
    e.rect = Some([0.35, 0.40, 0.30, 0.20]);
    e.strength = Some(0.03);
    e.roundness = Some(0.1);
    let mut doc = EditDoc::default();
    doc.effects = vec![e.clone()];
    let back: EditDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(back.effects, vec![e]);
}

#[test]
fn absent_mask_fields_are_not_written() {
    let s = serde_json::to_string(&spot()).unwrap();
    assert!(!s.contains("rect") && !s.contains("strength") && !s.contains("roundness"));
}

#[test]
fn the_mask_defaults_are_the_spec_numbers() {
    assert_eq!(
        (
            DEFAULT_BLUR,
            DEFAULT_PIXEL,
            DEFAULT_MASK_ROUNDNESS,
            DEFAULT_MASK_FEATHER
        ),
        (0.020, 0.018, 0.06, 0.010)
    );
}
