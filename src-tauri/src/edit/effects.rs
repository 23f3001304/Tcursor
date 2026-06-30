// Effect-region edit ops (add/update/remove), split out of api.rs so each file stays focused.
// v1 handles Spotlight regions; the kind set grows in later phases. `api::apply` delegates the
// effect ops here.
use crate::edit::api::EditOp;
use crate::edit::model::{EditDoc, EffectRegion};

fn next_effect_id(doc: &EditDoc) -> String {
    let n = doc.effects.iter()
        .filter_map(|e| e.id.strip_prefix('e').and_then(|s| s.parse::<u32>().ok()))
        .max().map(|m| m + 1).unwrap_or(doc.effects.len() as u32);
    format!("e{}", n)
}

/// Apply an effect-region op. No-op for non-effect ops (the match arm in `api::apply` only
/// routes the three effect variants here).
pub fn apply_effect(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::AddEffect { kind, start_ms, end_ms } => {
            let id = next_effect_id(doc);
            doc.effects.push(EffectRegion { id, kind, start_ms, end_ms });
        }
        EditOp::UpdateEffect { id, start_ms, end_ms } => {
            if let Some(e) = doc.effects.iter_mut().find(|e| e.id == id) {
                if let Some(v) = start_ms { e.start_ms = v; }
                if let Some(v) = end_ms { e.end_ms = v; }
            }
        }
        EditOp::RemoveEffect { id } => doc.effects.retain(|e| e.id != id),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
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
    fn add_effect_yields_distinct_ids() {
        let mut doc = empty();
        apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 100 });
        apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 200, end_ms: 300 });
        assert_ne!(doc.effects[0].id, doc.effects[1].id);
    }

    #[test]
    fn update_effect_changes_supplied_fields_only() {
        let mut doc = empty();
        apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 500 });
        let id = doc.effects[0].id.clone();
        apply_effect(&mut doc, EditOp::UpdateEffect { id, start_ms: Some(100), end_ms: None });
        assert_eq!((doc.effects[0].start_ms, doc.effects[0].end_ms), (100, 500));
    }

    #[test]
    fn remove_effect_drops_by_id() {
        let mut doc = empty();
        apply_effect(&mut doc, EditOp::AddEffect { kind: EffectKind::Spotlight, start_ms: 0, end_ms: 100 });
        let id = doc.effects[0].id.clone();
        apply_effect(&mut doc, EditOp::RemoveEffect { id });
        assert_eq!(doc.effects.len(), 0);
    }
}
