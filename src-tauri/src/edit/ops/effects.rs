use crate::edit::model::{EditDoc, EffectKind, EffectRegion};
use crate::edit::ops::api::EditOp;
use crate::edit::ops::region::{auto_layer, clamp_order, dur_bound};

fn next_effect_id(doc: &EditDoc) -> String {
    let n = doc
        .effects
        .iter()
        .filter_map(|e| e.id.strip_prefix('e').and_then(|s| s.parse::<u32>().ok()))
        .max()
        .map(|m| m + 1)
        .unwrap_or(doc.effects.len() as u32);
    format!("e{}", n)
}

pub fn lift_always_on_spotlight(doc: &mut EditDoc) -> bool {
    let dur = dur_bound(doc);
    if !doc.settings.clickfx.spotlight
        || dur == u32::MAX
        || doc
            .effects
            .iter()
            .any(|e| matches!(e.kind, EffectKind::Spotlight))
    {
        return false;
    }
    let id = next_effect_id(doc);
    doc.effects.push(EffectRegion {
        id,
        kind: EffectKind::Spotlight,
        start_ms: 0,
        end_ms: dur,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
    });
    doc.settings.clickfx.spotlight = false;
    true
}

pub fn apply_effect(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::AddEffect {
            kind,
            start_ms,
            end_ms,
        } => {
            let id = next_effect_id(doc);
            let dur = dur_bound(doc);
            let (start_ms, end_ms) = (start_ms.min(dur), end_ms.min(dur));
            let existing: Vec<(u32, u32, u32)> = doc
                .effects
                .iter()
                .map(|e| (e.start_ms, e.end_ms, e.layer))
                .collect();
            let layer = auto_layer(&existing, start_ms, end_ms);
            doc.effects.push(EffectRegion {
                id,
                kind,
                start_ms,
                end_ms,
                fade_in_ms: 250,
                fade_out_ms: 250,
                mode: None,
                dim: None,
                radius: None,
                feather: None,
                layer,
            });
        }
        EditOp::UpdateEffect {
            id,
            start_ms,
            end_ms,
            fade_in_ms,
            fade_out_ms,
            mode,
            dim,
            radius,
            feather,
            layer,
        } => {
            let dur = dur_bound(doc);
            if let Some(e) = doc.effects.iter_mut().find(|e| e.id == id) {
                if let Some(v) = start_ms {
                    e.start_ms = v.min(dur);
                }
                if let Some(v) = end_ms {
                    e.end_ms = v.min(dur);
                }
                clamp_order(&mut e.start_ms, &mut e.end_ms, start_ms.is_some());
                if let Some(v) = fade_in_ms {
                    e.fade_in_ms = v;
                }
                if let Some(v) = fade_out_ms {
                    e.fade_out_ms = v;
                }
                if let Some(s) = mode {
                    e.mode = match s.as_str() {
                        "global" | "default" | "none" => None,
                        "classic" => Some(crate::settings::model::SpotlightMode::Classic),
                        "blur" => Some(crate::settings::model::SpotlightMode::Blur),
                        "halo" => Some(crate::settings::model::SpotlightMode::Halo),
                        "breathing" => Some(crate::settings::model::SpotlightMode::Breathing),
                        "nebula" => Some(crate::settings::model::SpotlightMode::Nebula),
                        "vignette" => Some(crate::settings::model::SpotlightMode::Vignette),
                        _ => e.mode,
                    };
                }
                if let Some(v) = dim {
                    e.dim = if v < 0.0 { None } else { Some(v) };
                }
                if let Some(v) = radius {
                    e.radius = if v < 0.0 { None } else { Some(v) };
                }
                if let Some(v) = feather {
                    e.feather = if v < 0.0 { None } else { Some(v) };
                }
                if let Some(v) = layer {
                    e.layer = v;
                }
            }
        }
        EditOp::RemoveEffect { id } => doc.effects.retain(|e| e.id != id),
        _ => {}
    }
}

#[cfg(test)]
#[path = "effects_tests.rs"]
mod tests;
