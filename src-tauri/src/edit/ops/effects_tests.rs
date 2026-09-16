use super::*;
use crate::edit::model::EffectKind;
fn empty() -> EditDoc {
    EditDoc::default()
}

#[test]
fn add_effect_appends_with_id_and_span() {
    let mut doc = empty();
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 500,
            end_ms: 1500,
        },
    );
    assert_eq!(doc.effects.len(), 1);
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (500, 1500)
    );
    assert!(doc.effects[0].id.starts_with('e'));
}

#[test]
fn add_effect_clamps_end_to_trim_duration() {
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 800,
            end_ms: 2800,
        },
    );
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (800, 1000)
    );
}

#[test]
fn add_effect_yields_distinct_ids() {
    let mut doc = empty();
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 100,
        },
    );
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 200,
            end_ms: 300,
        },
    );
    assert_ne!(doc.effects[0].id, doc.effects[1].id);
}

#[test]
fn add_effect_auto_assigns_a_free_layer() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 1000,
        },
    );
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 500,
            end_ms: 1500,
        },
    );
    assert_eq!(doc.effects[0].layer, 0);
    assert_eq!(doc.effects[1].layer, 1);
}

#[test]
fn update_effect_changes_supplied_fields_only() {
    let mut doc = empty();
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 500,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(
        &mut doc,
        EditOp::UpdateEffect {
            id,
            start_ms: Some(100),
            end_ms: None,
            fade_in_ms: None,
            fade_out_ms: None,
            mode: None,
            dim: None,
            radius: None,
            feather: None,
            layer: None,
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (100, 500));
}

#[test]
fn update_effect_sets_fades() {
    let mut doc = EditDoc::default();
    doc.trim.out_ms = 2000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 1000,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(
        &mut doc,
        EditOp::UpdateEffect {
            id,
            start_ms: None,
            end_ms: None,
            fade_in_ms: Some(80),
            fade_out_ms: Some(300),
            mode: None,
            dim: None,
            radius: None,
            feather: None,
            layer: None,
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    assert_eq!(
        (doc.effects[0].fade_in_ms, doc.effects[0].fade_out_ms),
        (80, 300)
    );
}

#[test]
fn update_effect_sets_layer() {
    let mut doc = empty();
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 500,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(
        &mut doc,
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
            layer: Some(3),
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    assert_eq!(doc.effects[0].layer, 3);
}

#[test]
fn remove_effect_drops_by_id() {
    let mut doc = empty();
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 0,
            end_ms: 100,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(&mut doc, EditOp::RemoveEffect { id });
    assert_eq!(doc.effects.len(), 0);
}

#[test]
fn update_effect_start_past_end_pulls_end_to_match() {
    let mut doc = empty();
    doc.trim.out_ms = 10_000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 1000,
            end_ms: 2000,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(
        &mut doc,
        EditOp::UpdateEffect {
            id,
            start_ms: Some(8000),
            end_ms: None,
            fade_in_ms: None,
            fade_out_ms: None,
            mode: None,
            dim: None,
            radius: None,
            feather: None,
            layer: None,
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (8000, 8000)
    );
}

#[test]
fn update_effect_end_before_start_pulls_start_to_match() {
    let mut doc = empty();
    doc.trim.out_ms = 10_000;
    apply_effect(
        &mut doc,
        EditOp::AddEffect {
            kind: EffectKind::Spotlight,
            start_ms: 5000,
            end_ms: 6000,
        },
    );
    let id = doc.effects[0].id.clone();
    apply_effect(
        &mut doc,
        EditOp::UpdateEffect {
            id,
            start_ms: None,
            end_ms: Some(1000),
            fade_in_ms: None,
            fade_out_ms: None,
            mode: None,
            dim: None,
            radius: None,
            feather: None,
            layer: None,
            rect: None,
            strength: None,
            roundness: None,
        },
    );
    assert_eq!(
        (doc.effects[0].start_ms, doc.effects[0].end_ms),
        (1000, 1000)
    );
}

#[path = "effects_lift_tests.rs"]
mod lift_tests;
