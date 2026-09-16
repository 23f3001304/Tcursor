use crate::edit::model::{EditDoc, EffectKind};
use crate::edit::ops::api::EditOp;
use crate::edit::ops::effects::{apply_effect, clamp_rect, MASK_SEED_RECT};

fn with_mask() -> EditDoc {
    let mut doc = EditDoc::default();
    doc.trim.out_ms = 10_000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Blur,
            start_ms: 1000,
            end_ms: 3000,
        },
    );
    doc
}

fn update(
    doc: &mut EditDoc,
    rect: Option<[f32; 4]>,
    strength: Option<f32>,
    roundness: Option<f32>,
) {
    let id = doc.effects[0].id.clone();
    apply_effect(
        doc,
        EditOp::UpdateEffect {
            id,
            start_ms: None,
            end_ms: None,
            fade_in_ms: None,
            fade_out_ms: None,
            mode: None,
            dim: None,
            radius: None,
            feather: None,
            layer: None,
            rect,
            strength,
            roundness,
        },
    );
}

#[test]
fn adding_a_mask_seeds_the_centred_rect_and_a_spotlight_gets_none() {
    let doc = with_mask();
    assert_eq!(doc.effects[0].rect, Some(MASK_SEED_RECT));
    let mut spot = EditDoc::default();
    apply_effect(
        &mut spot,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 100,
        },
    );
    assert_eq!(spot.effects[0].rect, None);
}

#[test]
fn a_rect_is_clamped_so_it_never_leaves_the_canvas() {
    assert_eq!(clamp_rect([0.9, 0.9, 0.5, 0.5]), Some([0.5, 0.5, 0.5, 0.5]));
    assert_eq!(
        clamp_rect([-0.2, 0.1, 2.0, 0.005]),
        Some([0.0, 0.1, 1.0, 0.01])
    );
    assert_eq!(clamp_rect([f32::NAN, 0.1, 0.2, 0.2]), None);
    let mut doc = with_mask();
    update(&mut doc, Some([f32::INFINITY, 0.0, 0.1, 0.1]), None, None);
    assert_eq!(doc.effects[0].rect, Some(MASK_SEED_RECT));
}

#[test]
fn strength_and_roundness_are_clamped_to_their_ranges() {
    let mut doc = with_mask();
    update(&mut doc, None, Some(0.5), Some(0.9));
    assert_eq!(
        (doc.effects[0].strength, doc.effects[0].roundness),
        (Some(0.120), Some(0.5))
    );
    update(&mut doc, None, Some(0.0), Some(-1.0));
    assert_eq!(
        (doc.effects[0].strength, doc.effects[0].roundness),
        (Some(0.002), Some(0.0))
    );
}

#[test]
fn a_rect_is_ignored_on_a_spotlight_and_mode_and_radius_on_a_mask() {
    let mut spot = EditDoc::default();
    apply_effect(
        &mut spot,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 100,
        },
    );
    update(&mut spot, Some([0.1, 0.1, 0.2, 0.2]), None, None);
    assert_eq!(spot.effects[0].rect, None);
    let mut mask = with_mask();
    let id = mask.effects[0].id.clone();
    apply_effect(
        &mut mask,
        EditOp::UpdateEffect {
            id,
            start_ms: None,
            end_ms: None,
            fade_in_ms: None,
            fade_out_ms: None,
            mode: Some("halo".into()),
            dim: Some(0.3),
            radius: Some(0.2),
            feather: Some(0.02),
            layer: None,
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    let e = &mask.effects[0];
    assert_eq!((e.mode, e.radius), (None, None));
    assert_eq!((e.dim, e.feather), (Some(0.3), Some(0.02)));
}

#[test]
fn update_effect_parses_with_the_mask_fields_absent() {
    let op: EditOp = serde_json::from_str(r#"{"op":"update_effect","id":"e0","layer":2}"#).unwrap();
    assert!(matches!(
        op,
        EditOp::UpdateEffect {
            rect: None,
            strength: None,
            roundness: None,
            ..
        }
    ));
}
