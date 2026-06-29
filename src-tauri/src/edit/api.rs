use serde::{Deserialize, Serialize};
use crate::edit::model::{Cut, EditDoc, Speed, Trim, Zoom, ZoomTarget};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum EditOp {
    AddZoom { at_ms: u32, dur_ms: u32 },
    AddZoomFull { at_ms: u32, dur_ms: u32, scale: f32 },
    UpdateZoom {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        scale: Option<f32>,
        target: Option<ZoomTarget>,
        easing: Option<String>,
    },
    RemoveZoom { id: String },
    SetTrim { in_ms: u32, out_ms: u32 },
    AddCut { start_ms: u32, end_ms: u32 },
    SetSpeed { start_ms: u32, end_ms: u32, factor: f32 },
    SetLayoutSeg { id: String, layout: String },
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

pub fn apply(doc: &mut EditDoc, op: EditOp) {
    match op {
        EditOp::AddZoom { at_ms, dur_ms } => {
            let id = next_zoom_id(doc);
            doc.zooms.push(Zoom {
                id,
                start_ms: at_ms,
                end_ms: at_ms.saturating_add(dur_ms),
                target: ZoomTarget::Cursor,
                scale: 2.0,
                easing: "smooth".into(),
            });
        }
        EditOp::AddZoomFull { at_ms, dur_ms, scale } => {
            let id = next_zoom_id(doc);
            doc.zooms.push(Zoom { id, start_ms: at_ms, end_ms: at_ms.saturating_add(dur_ms),
                target: ZoomTarget::Cursor, scale, easing: "smooth".into() });
        }
        EditOp::UpdateZoom { id, start_ms, end_ms, scale, target, easing } => {
            if let Some(z) = doc.zooms.iter_mut().find(|z| z.id == id) {
                if let Some(v) = start_ms { z.start_ms = v; }
                if let Some(v) = end_ms { z.end_ms = v; }
                if let Some(v) = scale { z.scale = v; }
                if let Some(v) = target { z.target = v; }
                if let Some(v) = easing { z.easing = v; }
            }
        }
        EditOp::RemoveZoom { id } => {
            doc.zooms.retain(|z| z.id != id);
        }
        EditOp::SetTrim { in_ms, out_ms } => {
            doc.trim = Trim { in_ms, out_ms };
        }
        EditOp::AddCut { start_ms, end_ms } => {
            doc.cuts.push(Cut { start_ms, end_ms });
        }
        EditOp::SetSpeed { start_ms, end_ms, factor } => {
            let id = next_speed_id(doc);
            doc.speed.push(Speed { id, start_ms, end_ms, factor });
        }
        EditOp::SetLayoutSeg { id, layout } => {
            if let Some(seg) = doc.layout.iter_mut().find(|s| s.id == id) {
                seg.layout = layout;
            }
        }
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
mod tests {
    use super::*;
    use crate::edit::model::{EditDoc, Trim, LayoutSeg};

    fn empty() -> EditDoc { EditDoc::default() }

    #[test]
    fn add_zoom_appends_with_correct_span() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoom { at_ms: 500, dur_ms: 1000 });
        let z = &doc.zooms[0];
        assert_eq!((z.start_ms, z.end_ms, z.scale, z.easing.as_str()), (500, 1500, 2.0, "smooth"));
    }

    #[test]
    fn add_zoom_yields_distinct_ids() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 100 });
        apply(&mut doc, EditOp::AddZoom { at_ms: 200, dur_ms: 100 });
        assert_ne!(doc.zooms[0].id, doc.zooms[1].id);
        assert!(doc.zooms[0].id.starts_with('z'));
    }

    #[test]
    fn update_zoom_changes_only_supplied_fields() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 500 });
        let id = doc.zooms[0].id.clone();
        apply(&mut doc, EditOp::UpdateZoom {
            id, start_ms: Some(100), end_ms: None,
            scale: None, target: None, easing: None,
        });
        assert_eq!((doc.zooms[0].start_ms, doc.zooms[0].end_ms, doc.zooms[0].scale), (100, 500, 2.0));
    }

    #[test]
    fn update_zoom_unknown_id_is_noop() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 100 });
        apply(&mut doc, EditOp::UpdateZoom {
            id: "z999".into(), start_ms: Some(9999), end_ms: None,
            scale: None, target: None, easing: None,
        });
        assert_eq!(doc.zooms[0].start_ms, 0);
    }

    #[test]
    fn remove_zoom_drops_by_id() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoom { at_ms: 0, dur_ms: 100 });
        apply(&mut doc, EditOp::AddZoom { at_ms: 200, dur_ms: 100 });
        let id = doc.zooms[0].id.clone(); apply(&mut doc, EditOp::RemoveZoom { id });
        assert_eq!(doc.zooms.len(), 1);
    }

    #[test]
    fn set_trim_replaces_trim() {
        let mut doc = empty();
        apply(&mut doc, EditOp::SetTrim { in_ms: 200, out_ms: 8000 });
        assert_eq!(doc.trim, Trim { in_ms: 200, out_ms: 8000 });
    }

    #[test]
    fn metrics_kept_ms_subtracts_cuts() {
        let mut doc = empty();
        apply(&mut doc, EditOp::SetTrim { in_ms: 0, out_ms: 10000 });
        apply(&mut doc, EditOp::AddCut { start_ms: 1000, end_ms: 3000 });
        let m = metrics(&doc);
        assert_eq!((m.duration_ms, m.kept_ms, m.cut_count), (10000, 8000, 1));
    }

    #[test]
    fn set_layout_seg_noop_unknown() {
        let mut doc = empty();
        doc.layout.push(LayoutSeg { id: "l1".into(), start_ms: 0, end_ms: 1000, layout: "screen".into() });
        apply(&mut doc, EditOp::SetLayoutSeg { id: "l99".into(), layout: "pip".into() });
        assert_eq!(doc.layout[0].layout, "screen");
    }

    #[test]
    fn add_zoom_full_uses_given_scale() {
        let mut doc = empty();
        apply(&mut doc, EditOp::AddZoomFull { at_ms: 100, dur_ms: 500, scale: 3.0 });
        let z = &doc.zooms[0];
        assert_eq!((z.start_ms, z.end_ms, z.scale), (100, 600, 3.0));
        assert!(z.id.starts_with('z'));
    }
}
