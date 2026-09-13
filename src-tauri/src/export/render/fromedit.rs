// Inverse of `edit::seed`: rebuild the exporter's raw `ZoomRegion`s and the
// `SetLayout` action track from a persisted `EditDoc`, so export renders the saved
// plan instead of regenerating it. A seeded doc round-trips byte-identical (proven
// by the test below). Trim/cuts/speed arrive already applied: `render_edit::EditState::load` runs
// the doc through `edit::remap_doc` first, so every span here is on the output clock.
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, Zoom, ZoomTarget};
use crate::export::types::{Easing, FramePoint, ZoomRegion, SPRING_DEFAULT};

/// Map an easing wire-name back to `Easing`. The bare word `"spring"` carries no parameters, so
/// it means `SPRING_DEFAULT`; a `spring(stiffness,damping[,mass])` string carries its own, exactly
/// like `cubic(...)`. Anything the string cannot reconstruct falls back to `cfg` (only the tuned
/// default `Smooth` is ever seeded).
pub fn easing_from(name: &str, cfg_easing: Easing) -> Easing {
    match name {
        "smooth" => Easing::Smooth,
        "linear" => Easing::Linear,
        "spring" => SPRING_DEFAULT,
        "ease_in" => Easing::EaseIn,
        "ease_out" => Easing::EaseOut,
        "ease_in_out" => Easing::EaseInOut,
        // A parameterised string carries its whole curve, so it reconstructs exactly.
        _ => crate::export::spring::parse_spring(name)
            .map(|(stiffness, damping, mass)| Easing::Spring { stiffness, damping, mass })
            .or_else(|| crate::export::cubic::parse_cubic(name)
                .map(|(x1, y1, x2, y2)| Easing::Cubic { x1, y1, x2, y2 }))
            .unwrap_or(cfg_easing), // unknown -> config's easing
    }
}

/// Anchor for a zoom: `Fixed` carries the screen-local press point the seed stored;
/// `Cursor` (only a user- or AI-added target; seeded docs are all `Fixed`) has no stored
/// point, so default to screen center (`anchor_regions` then re-anchors into panel).
/// A `Cursor` region also sets `follow_cursor`, so `CameraSim` aims at the live cursor
/// and never reads this fallback - it exists only to keep the field total.
fn anchor_for(z: &Zoom, sw: u32, sh: u32) -> FramePoint {
    match z.target {
        ZoomTarget::Fixed { x, y } => {
            let px = if x <= 1.0 && x >= 0.0 { x * sw as f32 } else { x };
            let py = if y <= 1.0 && y >= 0.0 { y * sh as f32 } else { y };
            FramePoint { x: px as i32, y: py as i32 }
        }
        ZoomTarget::Cursor => FramePoint { x: sw as i32 / 2, y: sh as i32 / 2 },
    }
}

/// PURE inverse of `seed::zooms_from_regions`: one raw `ZoomRegion` per `Zoom`.
/// `zoom_in_ms`/`zoom_out_ms` are read straight off each `Zoom` (per-region editable);
/// `cfg` is kept only as the `easing_from` fallback. `sw`/`sh` only matter for the
/// `Cursor` fallback; `Fixed` anchors (every seeded zoom) are reproduced exactly.
pub fn regions_from_doc(doc: &EditDoc, sw: u32, sh: u32) -> Vec<ZoomRegion> {
    let cfg = doc.settings.zoom.to_zoom_config();
    doc.zooms
        .iter()
        .map(|z| ZoomRegion {
            start_ms: z.start_ms,
            end_ms: z.end_ms,
            zoom_in_ms: z.zoom_in_ms,
            zoom_out_ms: z.zoom_out_ms,
            target_scale: z.scale,
            anchor: anchor_for(z, sw, sh),
            easing: easing_from(&z.easing, cfg.easing),
            layer: z.layer,
            cam_action: z.cam_action,
            follow_cursor: matches!(z.target, ZoomTarget::Cursor),
        })
        .collect()
}

/// Parse a `LayoutSeg.layout` wire-name (e.g. `screen_only`) back to a `LayoutId`,
/// the same lowercase serde form `seed::layout_name` writes; unknown -> `Screen`.
pub fn layout_id_from(name: &str) -> LayoutId {
    serde_json::from_value(serde_json::Value::String(name.to_string()))
        .unwrap_or(LayoutId::Screen)
}

/// PURE inverse of `seed::layout_from_actions`: rebuild the `SetLayout` action track
/// that `LayoutTrack::new` consumes (it injects its own `(0, Screen)` base, then one
/// switch per action at `start_ms`). Returns `None` when `doc.layout` is empty -- the
/// caller's signal to FALL BACK to the recorded `actions.json` track.
pub fn layout_segs_from_doc(doc: &EditDoc) -> Option<Vec<ActionEvent>> {
    if doc.layout.is_empty() {
        return None;
    }
    Some(
        doc.layout
            .iter()
            .map(|s| ActionEvent { t: s.start_ms, kind: ActionKind::SetLayout(layout_id_from(&s.layout)) })
            .collect(),
    )
}

#[cfg(test)]
#[path = "fromedit_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "fromedit_spring_tests.rs"]
mod spring_tests;
