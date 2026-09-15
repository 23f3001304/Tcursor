use crate::edit::model::{CameraMove, EditDoc, Trim, ZoomTarget, DEFAULT_CAM_ROUNDNESS};
use crate::edit::ops::ids::{next_cam_id, next_layout_id};
use crate::edit::ops::motion;
use crate::edit::ops::region::{
    clamp_order, dur_bound, valid_cam_shape, valid_easing, valid_layout, CAM_KF_SNAP_MS,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum EditOp {
    AddZoom {
        at_ms: u32,
        dur_ms: u32,
    },
    AddZoomFull {
        at_ms: u32,
        dur_ms: u32,
        scale: f32,
    },
    UpdateZoom {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        scale: Option<f32>,
        target: Option<ZoomTarget>,
        easing: Option<String>,
        zoom_in_ms: Option<u32>,
        zoom_out_ms: Option<u32>,
        layer: Option<u32>,
        smart_typing: Option<bool>,
        #[serde(default)]
        easing_out: Option<String>,
    },
    RemoveZoom {
        id: String,
    },
    ClearZooms,
    SetZoomCamAction {
        id: String,
        action: Option<crate::settings::model::CamZoomAction>,
    },
    SetTrim {
        in_ms: u32,
        out_ms: u32,
    },
    SetAspect {
        aspect: crate::export::types::Aspect,
    },
    AddCut {
        start_ms: u32,
        end_ms: u32,
    },
    AddCuts {
        spans: Vec<(u32, u32)>,
    },
    UpdateCut {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
    },
    RemoveCut {
        id: String,
    },
    SetSpeed {
        start_ms: u32,
        end_ms: u32,
        factor: f32,
    },
    UpdateSpeed {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        factor: Option<f32>,
    },
    RemoveSpeed {
        id: String,
    },
    AddLayoutSeg {
        at_ms: u32,
        dur_ms: u32,
        layout: String,
        transition_out_ms: Option<u32>,
        easing_out: Option<String>,
    },
    UpdateLayoutSeg {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        layout: Option<String>,
        transition_ms: Option<u32>,
        easing: Option<String>,
        transition_out_ms: Option<u32>,
        easing_out: Option<String>,
    },
    RemoveLayoutSeg {
        id: String,
    },
    SetArrangement {
        id: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "crate::edit::ops::arrangement::double_option"
        )]
        screen: Option<Option<crate::edit::model::PanelPose>>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "crate::edit::ops::arrangement::double_option"
        )]
        cam: Option<Option<crate::edit::model::PanelPose>>,
    },
    ClearArrangement {
        id: String,
    },
    AddEffect {
        kind: crate::edit::model::EffectKind,
        start_ms: u32,
        end_ms: u32,
    },
    UpdateEffect {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        fade_in_ms: Option<u32>,
        fade_out_ms: Option<u32>,
        mode: Option<String>,
        dim: Option<f32>,
        radius: Option<f32>,
        feather: Option<f32>,
        layer: Option<u32>,
    },
    RemoveEffect {
        id: String,
    },
    AddCameraMove {
        t_ms: u32,
        x: f32,
        y: f32,
        size: f32,
        #[serde(default)]
        shape: Option<String>,
        #[serde(default)]
        roundness: Option<f32>,
    },
    UpdateCameraMove {
        id: String,
        t_ms: Option<u32>,
        x: Option<f32>,
        y: Option<f32>,
        size: Option<f32>,
        easing: Option<String>,
        #[serde(default)]
        shape: Option<String>,
        #[serde(default)]
        roundness: Option<f32>,
    },
    RemoveCameraMove {
        id: String,
    },
    ApplyMotionDefault,
    UpdateCaption {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        text: Option<String>,
    },
    RemoveCaption {
        id: String,
    },
    MergeCaptions {
        id: String,
    },
    SplitCaption {
        id: String,
        at_ms: u32,
    },
    SetCaptions {
        captions: Vec<crate::edit::captions::Caption>,
    },
    ClearCaptions,
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

pub const NEW_LAYOUT_TRANSITION_MS: u32 = 350;

pub fn apply(doc: &mut EditDoc, op: EditOp) {
    if crate::edit::ops::timeops::apply_time_op(doc, &op) {
        return;
    }
    match op {
        EditOp::AddZoom { at_ms, dur_ms } => motion::add_zoom(doc, at_ms, dur_ms, 2.0),
        EditOp::AddZoomFull {
            at_ms,
            dur_ms,
            scale,
        } => motion::add_zoom(doc, at_ms, dur_ms, scale),
        EditOp::UpdateZoom {
            id,
            start_ms,
            end_ms,
            scale,
            target,
            easing,
            zoom_in_ms,
            zoom_out_ms,
            layer,
            smart_typing,
            easing_out,
        } => {
            let dur = dur_bound(doc);
            if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) {
                if let Some(v) = start_ms {
                    z.start_ms = v.min(dur);
                }
                if let Some(v) = end_ms {
                    z.end_ms = v.min(dur);
                }
                clamp_order(&mut z.start_ms, &mut z.end_ms, start_ms.is_some());
                if let Some(v) = scale {
                    z.scale = v;
                }
                if let Some(v) = target {
                    z.target = v;
                }
                if let Some(v) = easing {
                    z.easing = valid_easing(&v);
                }
                if let Some(v) = easing_out {
                    motion::set_zoom_easing_out(z, &v);
                }
                if let Some(v) = zoom_in_ms {
                    z.zoom_in_ms = v;
                }
                if let Some(v) = zoom_out_ms {
                    z.zoom_out_ms = v;
                }
                if let Some(v) = layer {
                    z.layer = v;
                }
                if let Some(v) = smart_typing {
                    z.smart_typing = v;
                }
            }
        }
        EditOp::RemoveZoom { id } => {
            doc.zooms.retain(|z| z.id != id);
        }
        EditOp::ClearZooms => doc.zooms.clear(),
        EditOp::SetZoomCamAction { id, action } => {
            if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) {
                z.cam_action = action;
            }
        }
        EditOp::SetTrim { in_ms, out_ms } => {
            doc.trim = Trim { in_ms, out_ms };
        }
        EditOp::SetAspect { aspect } => {
            doc.aspect = aspect;
        }
        EditOp::AddCut { .. }
        | EditOp::AddCuts { .. }
        | EditOp::UpdateCut { .. }
        | EditOp::RemoveCut { .. }
        | EditOp::SetSpeed { .. }
        | EditOp::UpdateSpeed { .. }
        | EditOp::RemoveSpeed { .. } => unreachable!("handled by timeops"),
        EditOp::AddLayoutSeg {
            at_ms,
            dur_ms,
            layout,
            transition_out_ms,
            easing_out,
        } => {
            let id = next_layout_id(doc);
            let (dur, (m_in, m_out)) = (
                crate::edit::ops::region::dur_bound(doc),
                motion::for_layout(doc),
            );
            doc.layout.push(crate::edit::model::LayoutSeg {
                id,
                start_ms: at_ms.min(dur),
                end_ms: at_ms.saturating_add(dur_ms).min(dur),
                layout: valid_layout(&layout),
                transition_ms: NEW_LAYOUT_TRANSITION_MS,
                easing: m_in,
                transition_out_ms: transition_out_ms.unwrap_or(NEW_LAYOUT_TRANSITION_MS),
                easing_out: easing_out.map_or(m_out, |v| valid_easing(&v)),
                arrangement: None,
            });
        }
        EditOp::UpdateLayoutSeg {
            id,
            start_ms,
            end_ms,
            layout,
            transition_ms,
            easing,
            transition_out_ms,
            easing_out,
        } => {
            let dur = crate::edit::ops::region::dur_bound(doc);
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) {
                if let Some(v) = start_ms {
                    s.start_ms = v.min(dur);
                }
                if let Some(v) = end_ms {
                    s.end_ms = v.min(dur);
                }
                clamp_order(&mut s.start_ms, &mut s.end_ms, start_ms.is_some());
                if let Some(v) = layout {
                    s.layout = valid_layout(&v);
                }
                if let Some(v) = transition_ms {
                    s.transition_ms = v;
                }
                if let Some(v) = easing {
                    s.easing = valid_easing(&v);
                }
                if let Some(v) = transition_out_ms {
                    s.transition_out_ms = v;
                }
                if let Some(v) = easing_out {
                    s.easing_out = valid_easing(&v);
                }
            }
        }
        EditOp::RemoveLayoutSeg { id } => {
            doc.layout.retain(|s| s.id != id);
        }
        op @ (EditOp::SetArrangement { .. } | EditOp::ClearArrangement { .. }) => {
            crate::edit::ops::arrangement::apply_arrangement(doc, op)
        }
        op @ (EditOp::AddEffect { .. }
        | EditOp::UpdateEffect { .. }
        | EditOp::RemoveEffect { .. }) => crate::edit::ops::effects::apply_effect(doc, op),
        EditOp::AddCameraMove {
            t_ms,
            x,
            y,
            size,
            shape,
            roundness,
        } => {
            let (t_ms, cam_easing) = (t_ms.min(dur_bound(doc)), motion::for_camera(doc));
            let (shape, roundness) = (
                shape.map(|s| valid_cam_shape(&s)),
                roundness.map(|r| r.clamp(0.0, 0.5)),
            );
            if let Some(m) = doc
                .camera_moves
                .iter_mut()
                .filter(|m| m.t_ms.abs_diff(t_ms) <= CAM_KF_SNAP_MS)
                .min_by_key(|m| m.t_ms.abs_diff(t_ms))
            {
                (m.x, m.y, m.size) = (clamp01(x), clamp01(y), clamp01(size));
                if let Some(s) = shape {
                    m.shape = s;
                }
                if let Some(r) = roundness {
                    m.roundness = r;
                }
                return;
            }
            let id = next_cam_id(doc);
            doc.camera_moves.push(CameraMove {
                id,
                t_ms,
                x: clamp01(x),
                y: clamp01(y),
                size: clamp01(size),
                easing: cam_easing,
                shape: shape.unwrap_or_else(|| "layout".into()),
                roundness: roundness.unwrap_or(DEFAULT_CAM_ROUNDNESS),
            });
            doc.camera_moves.sort_by_key(|m| m.t_ms);
        }
        EditOp::UpdateCameraMove {
            id,
            t_ms,
            x,
            y,
            size,
            easing,
            shape,
            roundness,
        } => {
            let dur = dur_bound(doc);
            let mut resort = false;
            if let Some(m) = doc.camera_moves.iter_mut().find(|m| m.id == id) {
                if let Some(v) = t_ms {
                    m.t_ms = v.min(dur);
                    resort = true;
                }
                if let Some(v) = x {
                    m.x = clamp01(v);
                }
                if let Some(v) = y {
                    m.y = clamp01(v);
                }
                if let Some(v) = size {
                    m.size = clamp01(v);
                }
                if let Some(v) = easing {
                    m.easing = valid_easing(&v);
                }
                if let Some(v) = shape {
                    m.shape = valid_cam_shape(&v);
                }
                if let Some(v) = roundness {
                    m.roundness = v.clamp(0.0, 0.5);
                }
            }
            if resort {
                doc.camera_moves.sort_by_key(|m| m.t_ms);
            }
        }
        EditOp::RemoveCameraMove { id } => {
            doc.camera_moves.retain(|m| m.id != id);
        }
        EditOp::ApplyMotionDefault => motion::apply_default(doc),
        op @ (EditOp::UpdateCaption { .. }
        | EditOp::RemoveCaption { .. }
        | EditOp::MergeCaptions { .. }
        | EditOp::SplitCaption { .. }
        | EditOp::SetCaptions { .. }
        | EditOp::ClearCaptions) => crate::edit::ops::captions::apply_caption(doc, op),
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
