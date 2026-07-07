// Inverse of `edit::seed`: rebuild the exporter's raw `ZoomRegion`s and the
// `SetLayout` action track from a persisted `EditDoc`, so export renders the saved
// plan instead of regenerating it. A seeded doc round-trips byte-identical (proven
// by the test below). Trim/cuts/speed are intentionally not applied here.
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, Zoom, ZoomTarget};
use crate::export::types::{Easing, FramePoint, ZoomRegion};

/// Map an easing wire-name back to `Easing`. The `Spring` stiffness/damping params
/// are NOT carried by a `Zoom`, so fall back to `cfg` for anything the string cannot
/// reconstruct (only the tuned default `Smooth` is ever seeded).
pub fn easing_from(name: &str, cfg_easing: Easing) -> Easing {
    match name {
        "smooth" => Easing::Smooth,
        "linear" => Easing::Linear,
        "ease_in" => Easing::EaseIn,
        "ease_out" => Easing::EaseOut,
        "ease_in_out" => Easing::EaseInOut,
        _ => cfg_easing, // "spring" (params lost) or unknown -> config's easing
    }
}

/// Anchor for a zoom: `Fixed` carries the screen-local press point the seed stored;
/// `Cursor` (only a user-added target; seeded docs are all `Fixed`) has no stored
/// point, so default to screen center (`anchor_regions` then re-anchors into panel).
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
mod tests {
    use super::*;
    use crate::edit::model::EditDoc;
    use crate::edit::seed::{layout_from_actions, zooms_from_regions};
    use crate::export::types::{Easing, FramePoint, ZoomRegion};

    fn cfg_region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
        // zoom_in/out + easing taken from the DEFAULT ZoomConfig (what a seeded doc's
        // settings reproduce), so the rebuilt region must match these exactly.
        let cfg = crate::settings::model::Settings::default().zoom.to_zoom_config();
        ZoomRegion {
            start_ms: start, end_ms: end, zoom_in_ms: cfg.zoom_in_ms, zoom_out_ms: cfg.zoom_out_ms,
            target_scale: cfg.target_scale, anchor: FramePoint { x, y }, easing: cfg.easing, layer: 0,
        }
    }

    fn eq_region(a: &ZoomRegion, b: &ZoomRegion) -> bool {
        a.start_ms == b.start_ms && a.end_ms == b.end_ms && a.zoom_in_ms == b.zoom_in_ms
            && a.zoom_out_ms == b.zoom_out_ms && a.target_scale == b.target_scale && a.layer == b.layer
            && a.anchor == b.anchor && format!("{:?}", a.easing) == format!("{:?}", b.easing)
    }

    /// THE PROOF: regions -> seed::zooms_from_regions -> EditDoc -> regions_from_doc
    /// returns the original regions, field for field. Seed<->export is lossless.
    #[test]
    fn regions_round_trip_through_edit_doc() {
        let orig = vec![
            cfg_region(0, 2650, 100, 200),
            cfg_region(5000, 7650, 1500, 900),
            cfg_region(9000, 9450, 0, 0),
        ];
        let doc = EditDoc { zooms: zooms_from_regions(&orig), ..Default::default() };
        let rebuilt = regions_from_doc(&doc, 1920, 1080);
        assert_eq!(rebuilt.len(), orig.len());
        for (i, (a, b)) in rebuilt.iter().zip(orig.iter()).enumerate() {
            assert!(eq_region(a, b), "region {} drifted: {:?} != {:?}", i, a, b);
        }
    }

    #[test]
    fn layer_round_trips_through_edit_doc() {
        use crate::edit::model::{Zoom, ZoomTarget};
        let mut doc = EditDoc::default();
        doc.zooms.push(Zoom { id: "z0".into(), start_ms: 0, end_ms: 1000, target: ZoomTarget::Cursor,
            scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 3 });
        let regs = regions_from_doc(&doc, 1920, 1080);
        assert_eq!(regs[0].layer, 3);
    }

    #[test]
    fn empty_zooms_make_no_regions() {
        let doc = EditDoc::default();
        assert!(regions_from_doc(&doc, 1920, 1080).is_empty());
    }

    #[test]
    fn cursor_target_defaults_to_screen_center() {
        use crate::edit::model::{Zoom, ZoomTarget};
        let doc = EditDoc {
            zooms: vec![Zoom { id: "z0".into(), start_ms: 0, end_ms: 100,
                target: ZoomTarget::Cursor, scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 0 }],
            ..Default::default()
        };
        let r = regions_from_doc(&doc, 1920, 1080);
        assert_eq!(r[0].anchor, FramePoint { x: 960, y: 540 });
    }

    #[test]
    fn per_region_durations_flow_into_regions() {
        use crate::edit::model::{Zoom, ZoomTarget};
        let mut doc = EditDoc::default();
        doc.zooms.push(Zoom { id: "z0".into(), start_ms: 0, end_ms: 1000, target: ZoomTarget::Cursor,
            scale: 2.0, easing: "smooth".into(), zoom_in_ms: 120, zoom_out_ms: 640, layer: 0 });
        let regs = regions_from_doc(&doc, 800, 600);
        assert_eq!((regs[0].zoom_in_ms, regs[0].zoom_out_ms), (120, 640));
    }

    #[test]
    fn easing_unknown_or_spring_falls_back_to_config() {
        let cfg = crate::settings::model::Settings::default().zoom.to_zoom_config();
        assert_eq!(format!("{:?}", easing_from("spring", cfg.easing)), format!("{:?}", cfg.easing));
        assert_eq!(format!("{:?}", easing_from("smooth", cfg.easing)), format!("{:?}", Easing::Smooth));
        assert_eq!(format!("{:?}", easing_from("linear", cfg.easing)), format!("{:?}", Easing::Linear));
    }

    #[test]
    fn empty_layout_signals_fallback() {
        assert!(layout_segs_from_doc(&EditDoc::default()).is_none());
    }

    /// Layout round-trips: an action track -> seed::layout_from_actions -> segs ->
    /// layout_segs_from_doc yields a track whose SetLayout switches match the input.
    #[test]
    fn layout_segs_round_trip() {
        use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
        let acts = vec![
            ActionEvent { t: 1000, kind: ActionKind::SetLayout(LayoutId::Camera) },
            ActionEvent { t: 3000, kind: ActionKind::SetLayout(LayoutId::ScreenOnly) },
        ];
        let doc = EditDoc { layout: layout_from_actions(&acts, 8000), ..Default::default() };
        let rebuilt = layout_segs_from_doc(&doc).unwrap();
        // seed prepends an l0 (0, Screen) span; rebuilt emits a SetLayout(Screen)@0 for
        // it, then the two real switches. LayoutTrack injects its own (0, Screen), so the
        // extra is a benign duplicate; the real switches reproduce exactly.
        assert_eq!(rebuilt[0], ActionEvent { t: 0, kind: ActionKind::SetLayout(LayoutId::Screen) });
        assert_eq!(rebuilt[1], acts[0]);
        assert_eq!(rebuilt[2], acts[1]);
    }
}
