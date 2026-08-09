use serde::{Deserialize, Serialize};
use crate::edit::model::{CameraMove, Cut, EditDoc, Speed, Trim, Zoom, ZoomTarget};
use crate::edit::ops::region::{auto_layer, dur_bound, valid_easing, valid_layout};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum EditOp {
    AddZoom { at_ms: u32, dur_ms: u32 },
    AddZoomFull { at_ms: u32, dur_ms: u32, scale: f32 },
    UpdateZoom { id: String, start_ms: Option<u32>, end_ms: Option<u32>, scale: Option<f32>, target: Option<ZoomTarget>, easing: Option<String>, zoom_in_ms: Option<u32>, zoom_out_ms: Option<u32>, layer: Option<u32> },
    RemoveZoom { id: String },
    /// Remove every zoom at once - the AI director's opening "rethink" step (the frontend reveals it
    /// as the mechanical auto-zooms clearing before the smart ones land).
    ClearZooms,
    /// Set (`Some`) or clear (`None`) a zoom's webcam-on-zoom override. A dedicated op rather
    /// than a field on `UpdateZoom`, because that op's "field is None => leave unchanged"
    /// convention cannot express "clear back to inherit" without an `Option<Option<_>>`.
    SetZoomCamAction { id: String, action: Option<crate::settings::model::CamZoomAction> },
    SetTrim { in_ms: u32, out_ms: u32 },
    SetAspect { aspect: crate::export::types::Aspect },
    AddCut { start_ms: u32, end_ms: u32 },
    SetSpeed { start_ms: u32, end_ms: u32, factor: f32 },
    AddLayoutSeg { at_ms: u32, dur_ms: u32, layout: String, transition_out_ms: Option<u32>, easing_out: Option<String> },
    UpdateLayoutSeg { id: String, start_ms: Option<u32>, end_ms: Option<u32>, layout: Option<String>, transition_ms: Option<u32>, easing: Option<String>, transition_out_ms: Option<u32>, easing_out: Option<String> },
    RemoveLayoutSeg { id: String },
    AddEffect { kind: crate::edit::model::EffectKind, start_ms: u32, end_ms: u32 },
    UpdateEffect { id: String, start_ms: Option<u32>, end_ms: Option<u32>, fade_in_ms: Option<u32>, fade_out_ms: Option<u32>, mode: Option<String>, dim: Option<f32>, radius: Option<f32>, feather: Option<f32>, layer: Option<u32> },
    RemoveEffect { id: String },
    AddCameraMove { t_ms: u32, x: f32, y: f32, size: f32 },
    UpdateCameraMove { id: String, t_ms: Option<u32>, x: Option<f32>, y: Option<f32>, size: Option<f32>, easing: Option<String> },
    RemoveCameraMove { id: String },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Metrics {
    pub duration_ms: u32,
    pub kept_ms: u32,
    pub zoom_count: usize,
    pub cut_count: usize,
}

fn next_zoom_id(doc: &EditDoc) -> String {
    let n = doc.zooms.iter().filter_map(|z| {
        z.id.strip_prefix('z').and_then(|s| s.parse::<u32>().ok())
    }).max().map(|m| m + 1).unwrap_or(doc.zooms.len() as u32);
    format!("z{}", n)
}

fn next_speed_id(doc: &EditDoc) -> String {
    let n = doc.speed.iter().filter_map(|s| {
        s.id.strip_prefix('s').and_then(|d| d.parse::<u32>().ok())
    }).max().map(|m| m + 1).unwrap_or(doc.speed.len() as u32);
    format!("s{}", n)
}

fn next_layout_id(doc: &EditDoc) -> String {
    let n = doc.layout.iter().filter_map(|s| s.id.strip_prefix('l').and_then(|d| d.parse::<u32>().ok()))
        .max().map(|m| m + 1).unwrap_or(doc.layout.len() as u32);
    format!("l{}", n)
}
fn next_cam_id(doc: &EditDoc) -> String {
    let n = doc.camera_moves.iter().filter_map(|m| m.id.strip_prefix('k').and_then(|d| d.parse::<u32>().ok()))
        .max().map(|m| m + 1).unwrap_or(doc.camera_moves.len() as u32);
    format!("k{}", n)
}
fn clamp01(v: f32) -> f32 { v.clamp(0.0, 1.0) }

pub fn apply(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::AddZoom { at_ms, dur_ms } => {
            let id = next_zoom_id(doc);
            let dur = dur_bound(doc);
            let (start_ms, end_ms) = (at_ms.min(dur), at_ms.saturating_add(dur_ms).min(dur));
            let existing: Vec<(u32, u32, u32)> = doc.zooms.iter().map(|z| (z.start_ms, z.end_ms, z.layer)).collect();
            let layer = auto_layer(&existing, start_ms, end_ms);
            doc.zooms.push(Zoom { id, start_ms, end_ms,
                target: ZoomTarget::Cursor, scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer, cam_action: None });
        }
        EditOp::AddZoomFull { at_ms, dur_ms, scale } => {
            let id = next_zoom_id(doc);
            let dur = dur_bound(doc);
            let (start_ms, end_ms) = (at_ms.min(dur), at_ms.saturating_add(dur_ms).min(dur));
            let existing: Vec<(u32, u32, u32)> = doc.zooms.iter().map(|z| (z.start_ms, z.end_ms, z.layer)).collect();
            let layer = auto_layer(&existing, start_ms, end_ms);
            doc.zooms.push(Zoom { id, start_ms, end_ms,
                target: ZoomTarget::Cursor, scale, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer, cam_action: None });
        }
        EditOp::UpdateZoom { id, start_ms, end_ms, scale, target, easing, zoom_in_ms, zoom_out_ms, layer } => {
            let dur = dur_bound(doc);
            if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) {
                if let Some(v) = start_ms { z.start_ms = v.min(dur); }
                if let Some(v) = end_ms { z.end_ms = v.min(dur); }
                if let Some(v) = scale { z.scale = v; }
                if let Some(v) = target { z.target = v; }
                if let Some(v) = easing { z.easing = valid_easing(&v); }
                if let Some(v) = zoom_in_ms { z.zoom_in_ms = v; }
                if let Some(v) = zoom_out_ms { z.zoom_out_ms = v; }
                if let Some(v) = layer { z.layer = v; }
            }
        }
        EditOp::RemoveZoom { id } => {
            doc.zooms.retain(|z| z.id != id);
        }
        EditOp::ClearZooms => doc.zooms.clear(),
        EditOp::SetZoomCamAction { id, action } => {
            if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) { z.cam_action = action; }
        }
        EditOp::SetTrim { in_ms, out_ms } => {
            doc.trim = Trim { in_ms, out_ms };
        }
        EditOp::SetAspect { aspect } => {
            doc.aspect = aspect;
        }
        EditOp::AddCut { start_ms, end_ms } => {
            doc.cuts.push(Cut { start_ms, end_ms });
        }
        EditOp::SetSpeed { start_ms, end_ms, factor } => {
            let id = next_speed_id(doc);
            doc.speed.push(Speed { id, start_ms, end_ms, factor });
        }
        EditOp::AddLayoutSeg { at_ms, dur_ms, layout, transition_out_ms, easing_out } => {
            let id = next_layout_id(doc);
            let dur = crate::edit::ops::region::dur_bound(doc);
            doc.layout.push(crate::edit::model::LayoutSeg {
                id, start_ms: at_ms.min(dur), end_ms: at_ms.saturating_add(dur_ms).min(dur),
                layout: valid_layout(&layout), transition_ms: 350, easing: "smooth".into(),
                transition_out_ms: transition_out_ms.unwrap_or(0),
                easing_out: easing_out.map_or_else(|| "smooth".into(), |v| valid_easing(&v)) });
        }
        EditOp::UpdateLayoutSeg { id, start_ms, end_ms, layout, transition_ms, easing, transition_out_ms, easing_out } => {
            let dur = crate::edit::ops::region::dur_bound(doc);
            if let Some(s) = doc.layout.iter_mut().find(|s| s.id == id) {
                if let Some(v) = start_ms { s.start_ms = v.min(dur); }
                if let Some(v) = end_ms { s.end_ms = v.min(dur); }
                if let Some(v) = layout { s.layout = valid_layout(&v); }
                if let Some(v) = transition_ms { s.transition_ms = v; }
                if let Some(v) = easing { s.easing = valid_easing(&v); }
                if let Some(v) = transition_out_ms { s.transition_out_ms = v; }
                if let Some(v) = easing_out { s.easing_out = valid_easing(&v); }
            }
        }
        EditOp::RemoveLayoutSeg { id } => { doc.layout.retain(|s| s.id != id); }
        op @ (EditOp::AddEffect { .. } | EditOp::UpdateEffect { .. } | EditOp::RemoveEffect { .. }) =>
            crate::edit::ops::effects::apply_effect(doc, op),
        EditOp::AddCameraMove { t_ms, x, y, size } => {
            let id = next_cam_id(doc);
            let dur = dur_bound(doc);
            doc.camera_moves.push(CameraMove {
                id, t_ms: t_ms.min(dur), x: clamp01(x), y: clamp01(y), size: clamp01(size),
                easing: "smooth".into() });
            doc.camera_moves.sort_by_key(|m| m.t_ms);
        }
        EditOp::UpdateCameraMove { id, t_ms, x, y, size, easing } => {
            let dur = dur_bound(doc);
            let mut resort = false;
            if let Some(m) = doc.camera_moves.iter_mut().find(|m| m.id == id) {
                if let Some(v) = t_ms { m.t_ms = v.min(dur); resort = true; }
                if let Some(v) = x { m.x = clamp01(v); }
                if let Some(v) = y { m.y = clamp01(v); }
                if let Some(v) = size { m.size = clamp01(v); }
                if let Some(v) = easing { m.easing = valid_easing(&v); }
            }
            if resort { doc.camera_moves.sort_by_key(|m| m.t_ms); }
        }
        EditOp::RemoveCameraMove { id } => { doc.camera_moves.retain(|m| m.id != id); }
    }
}

pub fn metrics(doc: &EditDoc) -> Metrics {
    let duration_ms = doc.trim.out_ms;
    let trim_in = doc.trim.in_ms;
    let trim_out = doc.trim.out_ms;
    let trim_span = trim_out.saturating_sub(trim_in);
    let cut_sum: u32 = doc.cuts.iter().map(|c| {
        let s = c.start_ms.max(trim_in);
        let e = c.end_ms.min(trim_out);
        e.saturating_sub(s)
    }).sum();
    Metrics {
        duration_ms,
        kept_ms: trim_span.saturating_sub(cut_sum),
        zoom_count: doc.zooms.len(),
        cut_count: doc.cuts.len(),
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
