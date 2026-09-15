use crate::edit::model::{EditDoc, Zoom, ZoomTarget};
use crate::edit::ops::ids::next_zoom_id;
use crate::edit::ops::region::{auto_layer, dur_bound, valid_easing};

pub(crate) fn add_zoom(doc: &mut EditDoc, at_ms: u32, dur_ms: u32, scale: f32) {
    let (id, dur) = (next_zoom_id(doc), dur_bound(doc));
    let (start_ms, end_ms) = (at_ms.min(dur), at_ms.saturating_add(dur_ms).min(dur));
    let existing: Vec<(u32, u32, u32)> = doc
        .zooms
        .iter()
        .map(|z| (z.start_ms, z.end_ms, z.layer))
        .collect();
    let (layer, (easing, easing_out)) = (auto_layer(&existing, start_ms, end_ms), for_zoom(doc));
    doc.zooms.push(Zoom {
        id,
        start_ms,
        end_ms,
        target: ZoomTarget::Cursor,
        scale,
        easing,
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer,
        cam_action: None,
        smart_typing: false,
        easing_out,
    });
}

pub(crate) fn for_zoom(doc: &EditDoc) -> (String, Option<String>) {
    let (i, o) = (
        valid_easing(&doc.settings.motion.easing),
        valid_easing(&doc.settings.motion.easing_out),
    );
    let out = if o == i { None } else { Some(o) };
    (i, out)
}

pub(crate) fn for_layout(doc: &EditDoc) -> (String, String) {
    (
        valid_easing(&doc.settings.motion.easing),
        valid_easing(&doc.settings.motion.easing_out),
    )
}

pub(crate) fn for_camera(doc: &EditDoc) -> String {
    valid_easing(&doc.settings.motion.easing)
}

pub(crate) fn set_zoom_easing_out(z: &mut Zoom, v: &str) {
    if v.is_empty() {
        z.easing_out = None;
        return;
    }
    let o = valid_easing(v);
    z.easing_out = if o == z.easing { None } else { Some(o) };
}

pub(crate) fn apply_default(doc: &mut EditDoc) {
    let (zi, zo) = for_zoom(doc);
    let (li, lo) = for_layout(doc);
    let ci = for_camera(doc);
    for z in &mut doc.zooms {
        z.easing = zi.clone();
        z.easing_out = zo.clone();
    }
    for s in &mut doc.layout {
        s.easing = li.clone();
        s.easing_out = lo.clone();
    }
    for m in &mut doc.camera_moves {
        m.easing = ci.clone();
    }
}

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;
