// Build a default EditDoc from a recording, identical to today's export inputs.
// load_or_seed returns an existing edit.json or seeds one from the same auto-zoom,
// manual-zoom, layout-track and settings the exporter derives, then writes it.
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, LayoutSeg, Trim, Zoom, ZoomTarget};
use crate::export::types::{Easing, ZoomRegion};
use crate::session::paths::ProjectPaths;

/// Stable lowercase name for an easing curve (mirrors the export `Easing` enum).
/// NOTE: `Spring` carries stiffness/damping that a plain string cannot hold; the
/// default `to_zoom_config()` never produces `Spring`, so this is not hit today.
fn easing_str(e: Easing) -> String {
    match e {
        Easing::Smooth => "smooth",
        Easing::Linear => "linear",
        Easing::Spring { .. } => "spring",
    }
    .to_string()
}

/// PURE: one `Zoom` per region, ids `z0, z1, ...`. The region `anchor` (the click /
/// press point, screen-local) is the load-bearing zoom-in target in `CameraSim`, so
/// it is preserved as `ZoomTarget::Fixed{x,y}` (NOT dropped to `Cursor`) to keep a
/// re-render byte-identical. Output length always equals input length.
pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom> {
    regions
        .iter()
        .enumerate()
        .map(|(i, r)| Zoom {
            id: format!("z{}", i),
            start_ms: r.start_ms,
            end_ms: r.end_ms,
            target: ZoomTarget::Fixed { x: r.anchor.x as f32, y: r.anchor.y as f32 },
            scale: r.target_scale,
            easing: easing_str(r.easing),
        })
        .collect()
}

/// Serde wire name for a `LayoutId` (e.g. `screen`, `screen_only`).
fn layout_name(id: LayoutId) -> String {
    serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "screen".to_string())
}

/// PURE: one `LayoutSeg` per active span in the recorded `SetLayout` track (mirrors
/// `LayoutTrack`: always starts at `(0, Screen)`), each span running to the next
/// switch and the last to `dur_ms`. Empty track -> a single `screen` span [0, dur].
pub fn layout_from_actions(actions: &[ActionEvent], dur_ms: u32) -> Vec<LayoutSeg> {
    let mut switches: Vec<(u32, LayoutId)> = vec![(0, LayoutId::Screen)];
    for a in actions {
        if let ActionKind::SetLayout(id) = a.kind {
            switches.push((a.t, id));
        }
    }
    let mut segs = Vec::with_capacity(switches.len());
    for (i, &(start, id)) in switches.iter().enumerate() {
        let end = switches.get(i + 1).map(|&(s, _)| s).unwrap_or(dur_ms).max(start);
        segs.push(LayoutSeg { id: format!("l{}", i), start_ms: start, end_ms: end, layout: layout_name(id) });
    }
    segs
}

/// Return an existing `edit.json`, else build the default `EditDoc` from the
/// recording (same auto/manual zooms, layout track, settings and clip duration the
/// exporter uses) and write it to `paths.edit()` before returning.
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc {
    if let Some(doc) = EditDoc::load(&paths.edit()) {
        return doc;
    }
    let doc = build_default(paths);
    let _ = doc.save(&paths.edit());
    doc
}

/// Construct the default doc. Mirrors `exporter::export`'s input build exactly so a
/// seeded render matches today: settings snapshot -> `ZoomConfig`, auto + manual raw
/// zoom regions, the `SetLayout` track, and `[0, clip duration]` for the trim.
fn build_default(paths: &ProjectPaths) -> EditDoc {
    let settings: crate::settings::model::Settings = std::fs::read(paths.settings())
        .ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let cfg = settings.zoom.to_zoom_config();
    let log = match crate::events::model::EventLog::load(&paths.events()) {
        Ok(l) => l,
        Err(_) => return EditDoc { settings, ..Default::default() },
    };
    let actions = crate::actions::model::ActionLog::load(&paths.actions())
        .map(|a| a.actions).unwrap_or_default();
    let typing = crate::events::typing::TypingLog::load(&paths.typing()).ms;

    // Raw regions: auto click-zoom then manual hold-zoom (same order as the exporter;
    // re-anchoring into the screen panel is deferred to render-time, as today).
    let mut raw = if settings.zoom.enabled {
        crate::export::autozoom::generate(&log.events, &log.screen, &cfg, &typing, settings.zoom.smart_hold)
    } else {
        Vec::new()
    };
    raw.extend(crate::export::manual::from_actions(&actions, &log.events, &log.screen, &cfg));

    let dur_ms = clip_duration_ms(paths, &log);
    EditDoc {
        version: 1,
        trim: Trim { in_ms: 0, out_ms: dur_ms },
        cuts: vec![],
        zooms: zooms_from_regions(&raw),
        speed: vec![],
        layout: layout_from_actions(&actions, dur_ms),
        effects: vec![],
        settings,
    }
}

/// Clip duration (ms) the exporter derives: `video_end - video_start` from the
/// real capture timeline (frame timestamps).
fn clip_duration_ms(paths: &ProjectPaths, log: &crate::events::model::EventLog) -> u32 {
    let tl = crate::export::timeline::build_timeline(paths, log, 60);
    let start = tl.frames.first().copied().unwrap_or(0);
    let end = tl.frames.last().copied().unwrap_or(start).max(start + 1);
    (end - start) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
    use crate::export::types::{Easing, FramePoint, ZoomRegion};

    fn region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
        ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: 350, zoom_out_ms: 450,
            target_scale: 2.2, anchor: FramePoint { x, y }, easing: Easing::Smooth }
    }

    #[test]
    fn ids_are_deterministic_and_count_matches() {
        let regs = vec![region(0, 100, 10, 20), region(500, 900, 30, 40), region(2000, 2500, 5, 6)];
        let zs = zooms_from_regions(&regs);
        assert_eq!(zs.len(), regs.len());
        assert_eq!(zs[0].id, "z0");
        assert_eq!(zs[1].id, "z1");
        assert_eq!(zs[2].id, "z2");
    }

    #[test]
    fn empty_regions_make_no_zooms() {
        assert!(zooms_from_regions(&[]).is_empty());
    }

    #[test]
    fn fields_map_faithfully_and_anchor_is_preserved_as_fixed() {
        let zs = zooms_from_regions(&[region(120, 880, 400, 300)]);
        let z = &zs[0];
        assert_eq!(z.start_ms, 120);
        assert_eq!(z.end_ms, 880);
        assert_eq!(z.scale, 2.2);
        assert_eq!(z.easing, "smooth");
        assert_eq!(z.target, ZoomTarget::Fixed { x: 400.0, y: 300.0 });
    }

    #[test]
    fn easing_names_map() {
        assert_eq!(easing_str(Easing::Smooth), "smooth");
        assert_eq!(easing_str(Easing::Linear), "linear");
        assert_eq!(easing_str(Easing::Spring { stiffness: 1.0, damping: 1.0 }), "spring");
    }

    #[test]
    fn empty_layout_track_is_one_screen_span() {
        let segs = layout_from_actions(&[], 5000);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].id, "l0");
        assert_eq!(segs[0].start_ms, 0);
        assert_eq!(segs[0].end_ms, 5000);
        assert_eq!(segs[0].layout, "screen");
    }

    #[test]
    fn layout_switches_make_consecutive_spans_to_duration() {
        let acts = vec![
            ActionEvent { t: 1000, kind: ActionKind::SetLayout(LayoutId::Camera) },
            ActionEvent { t: 3000, kind: ActionKind::SetLayout(LayoutId::ScreenOnly) },
        ];
        let segs = layout_from_actions(&acts, 8000);
        assert_eq!(segs.len(), 3);
        assert_eq!((segs[0].start_ms, segs[0].end_ms, segs[0].layout.as_str()), (0, 1000, "screen"));
        assert_eq!((segs[1].start_ms, segs[1].end_ms, segs[1].layout.as_str()), (1000, 3000, "camera"));
        assert_eq!((segs[2].start_ms, segs[2].end_ms, segs[2].layout.as_str()), (3000, 8000, "screen_only"));
    }
}
