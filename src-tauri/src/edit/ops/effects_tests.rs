// Tests for edit::ops::effects, split into their own file so effects.rs stays under the size limit.
use super::*;
use crate::edit::model::EffectKind;
fn empty() -> EditDoc { EditDoc::default() }

#[test]
fn add_effect_appends_with_id_and_span() {
    let mut doc = empty();
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 500, end_ms: 1500 });
    assert_eq!(doc.effects.len(), 1);
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (500, 1500));
    assert!(doc.effects[0].id.starts_with('e'));
}

#[test]
fn add_effect_clamps_end_to_trim_duration() {
    // Same "out of bounds near the end" bug as zoom, for a spotlight added near the clip end.
    let mut doc = empty();
    doc.trim.out_ms = 1000;
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 800, end_ms: 2800 });
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (800, 1000));
}

#[test]
fn add_effect_yields_distinct_ids() {
    let mut doc = empty();
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 100 });
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 200, end_ms: 300 });
    assert_ne!(doc.effects[0].id, doc.effects[1].id);
}

#[test]
fn add_effect_auto_assigns_a_free_layer() {
    let mut doc = empty();
    doc.trim.out_ms = 10000;
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 1000 });
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 500, end_ms: 1500 }); // overlaps
    assert_eq!(doc.effects[0].layer, 0);
    assert_eq!(doc.effects[1].layer, 1);
}

#[test]
fn update_effect_changes_supplied_fields_only() {
    let mut doc = empty();
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 500 });
    let id = doc.effects[0].id.clone();
    apply_effect(&mut doc, EditOp::UpdateEffect { id, start_ms: Some(100), end_ms: None, fade_in_ms: None, fade_out_ms: None, mode: None, dim: None, radius: None, feather: None, layer: None });
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (100, 500));
}

#[test]
fn update_effect_sets_fades() {
    let mut doc = EditDoc::default(); doc.trim.out_ms = 2000;
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 1000 });
    let id = doc.effects[0].id.clone();
    apply_effect(&mut doc, EditOp::UpdateEffect { id, start_ms: None, end_ms: None, fade_in_ms: Some(80), fade_out_ms: Some(300), mode: None, dim: None, radius: None, feather: None, layer: None });
    assert_eq!((doc.effects[0].fade_in_ms, doc.effects[0].fade_out_ms), (80, 300));
}

#[test]
fn update_effect_sets_layer() {
    let mut doc = empty();
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 500 });
    let id = doc.effects[0].id.clone();
    apply_effect(&mut doc, EditOp::UpdateEffect { id, start_ms: None, end_ms: None, fade_in_ms: None, fade_out_ms: None, mode: None, dim: None, radius: None, feather: None, layer: Some(3) });
    assert_eq!(doc.effects[0].layer, 3);
}

#[test]
fn remove_effect_drops_by_id() {
    let mut doc = empty();
    apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 100 });
    let id = doc.effects[0].id.clone();
    apply_effect(&mut doc, EditOp::RemoveEffect { id });
    assert_eq!(doc.effects.len(), 0);
}

#[test]
fn lifts_always_on_spotlight_to_full_span_region_and_disables_toggle() {
    let mut doc = empty();
    doc.trim.out_ms = 8000;
    doc.settings.clickfx.spotlight = true;
    assert!(lift_always_on_spotlight(&mut doc)); // changed
    assert_eq!(doc.effects.len(), 1);
    assert!(matches!(doc.effects[0].kind, EffectKind::Spotlight));
    assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (0, 8000));
    assert!(!doc.settings.clickfx.spotlight); // toggle now off - the region is the single source
    assert!(!lift_always_on_spotlight(&mut doc)); // idempotent (a Spotlight region now exists)
}

#[test]
fn lift_is_noop_when_spotlight_toggle_off() {
    let mut doc = empty();
    doc.settings.clickfx.spotlight = false;
    assert!(!lift_always_on_spotlight(&mut doc));
    assert!(doc.effects.is_empty());
}

#[test]
fn lift_is_noop_on_zero_duration_doc() {
    let mut doc = empty(); // trim.out_ms == 0 (degenerate no-event-log doc)
    doc.settings.clickfx.spotlight = true;
    assert!(!lift_always_on_spotlight(&mut doc)); // no un-grabbable [0,0] region...
    assert!(doc.effects.is_empty());
    assert!(doc.settings.clickfx.spotlight); // ...and the toggle is left on, not silently disabled
}
