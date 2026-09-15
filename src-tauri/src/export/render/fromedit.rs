use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, Zoom, ZoomTarget};
use crate::export::types::{Easing, FramePoint, ZoomRegion, SPRING_DEFAULT};

pub fn easing_from(name: &str, cfg_easing: Easing) -> Easing {
    match name {
        "smooth" => Easing::Smooth,
        "linear" => Easing::Linear,
        "spring" => SPRING_DEFAULT,
        "ease_in" => Easing::EaseIn,
        "ease_out" => Easing::EaseOut,
        "ease_in_out" => Easing::EaseInOut,
        _ => crate::export::spring::parse_spring(name)
            .map(|(stiffness, damping, mass)| Easing::Spring {
                stiffness,
                damping,
                mass,
            })
            .or_else(|| {
                crate::export::cubic::parse_cubic(name).map(|(x1, y1, x2, y2)| Easing::Cubic {
                    x1,
                    y1,
                    x2,
                    y2,
                })
            })
            .or_else(|| crate::export::keys::parse_keys(name).map(Easing::Keys))
            .unwrap_or(cfg_easing),
    }
}

fn anchor_for(z: &Zoom, sw: u32, sh: u32) -> FramePoint {
    match z.target {
        ZoomTarget::Fixed { x, y } => {
            let px = if x <= 1.0 && x >= 0.0 {
                x * sw as f32
            } else {
                x
            };
            let py = if y <= 1.0 && y >= 0.0 {
                y * sh as f32
            } else {
                y
            };
            FramePoint {
                x: px as i32,
                y: py as i32,
            }
        }
        ZoomTarget::Cursor => FramePoint {
            x: sw as i32 / 2,
            y: sh as i32 / 2,
        },
    }
}

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
            easing_out: easing_from(
                z.easing_out.as_deref().unwrap_or(&z.easing),
                easing_from(&z.easing, cfg.easing),
            ),
            layer: z.layer,
            cam_action: z.cam_action,
            follow_cursor: matches!(z.target, ZoomTarget::Cursor),
        })
        .collect()
}

pub fn layout_id_from(name: &str) -> LayoutId {
    serde_json::from_value(serde_json::Value::String(name.to_string())).unwrap_or(LayoutId::Screen)
}

pub fn layout_segs_from_doc(doc: &EditDoc) -> Option<Vec<ActionEvent>> {
    if doc.layout.is_empty() {
        return None;
    }
    Some(
        doc.layout
            .iter()
            .map(|s| ActionEvent {
                t: s.start_ms,
                kind: ActionKind::SetLayout(layout_id_from(&s.layout)),
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "fromedit_spring_tests.rs"]
mod spring_tests;
#[cfg(test)]
#[path = "fromedit_tests.rs"]
mod tests;
