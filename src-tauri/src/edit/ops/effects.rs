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

pub const MASK_SEED_RECT: [f32; 4] = [0.35, 0.40, 0.30, 0.20];
const STRENGTH_RANGE: (f32, f32) = (0.002, 0.120);

pub fn clamp_rect(r: [f32; 4]) -> Option<[f32; 4]> {
    if r.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let w = r[2].clamp(0.01, 1.0);
    let h = r[3].clamp(0.01, 1.0);
    Some([r[0].clamp(0.0, 1.0 - w), r[1].clamp(0.0, 1.0 - h), w, h])
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
        rect: None,
        strength: None,
        roundness: None,
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
                rect: kind.is_mask().then_some(MASK_SEED_RECT),
                strength: None,
                roundness: None,
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
            rect,
            strength,
            roundness,
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
                if !e.kind.is_mask() {
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
                    if let Some(v) = radius {
                        e.radius = if v < 0.0 { None } else { Some(v) };
                    }
                }
                if let Some(v) = dim {
                    e.dim = if v < 0.0 { None } else { Some(v) };
                }
                if let Some(v) = feather {
                    e.feather = if v < 0.0 { None } else { Some(v) };
                }
                if let Some(v) = layer {
                    e.layer = v;
                }
                if e.kind.is_mask() {
                    if let Some(r) = rect.and_then(clamp_rect) {
                        e.rect = Some(r);
                    }
                    if let Some(v) = strength {
                        e.strength = Some(v.clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1));
                    }
                    if let Some(v) = roundness {
                        e.roundness = Some(v.clamp(0.0, 0.5));
                    }
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

#[cfg(test)]
#[path = "effects_mask_tests.rs"]
mod mask_tests;
