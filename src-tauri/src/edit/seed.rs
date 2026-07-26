// Build a default EditDoc from a recording, identical to today's export inputs.
// load_or_seed returns an existing edit.json or seeds one from the same auto-zoom,
// manual-zoom, layout-track and settings the exporter derives, then writes it.
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, EffectKind, EffectRegion, LayoutSeg, Trim, Zoom, ZoomTarget};
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
        Easing::EaseIn => "ease_in",
        Easing::EaseOut => "ease_out",
        Easing::EaseInOut => "ease_in_out",
    }
    .to_string()
}

/// PURE: one `Zoom` per region, ids `z0, z1, ...`. The region `anchor` (the click /
/// press point, screen-local) is the load-bearing zoom-in target in `CameraSim`, so
/// it is preserved as `ZoomTarget::Fixed{x,y}` (NOT dropped to `Cursor`) to keep a
/// re-render byte-identical. Output length always equals input length.
pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom> {
    regions.iter().enumerate().map(|(i, r)| Zoom {
        id: format!("z{}", i), start_ms: r.start_ms, end_ms: r.end_ms,
        target: ZoomTarget::Fixed { x: r.anchor.x as f32, y: r.anchor.y as f32 },
        scale: r.target_scale, easing: easing_str(r.easing),
        zoom_in_ms: r.zoom_in_ms, zoom_out_ms: r.zoom_out_ms, layer: r.layer,
        cam_action: r.cam_action,
    }).collect()
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
        segs.push(LayoutSeg { id: format!("l{}", i), start_ms: start, end_ms: end, layout: layout_name(id),
            transition_ms: 350, easing: "smooth".into() });
    }
    segs
}

/// Return an existing `edit.json`, else build the default `EditDoc` from the recording (same
/// auto/manual zooms, layout track, settings, clip duration the exporter uses). An always-on
/// spotlight is lifted to an editable region here (on fresh AND older docs); writes when changed.
pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc {
    let (mut doc, fresh) = match EditDoc::load(&paths.edit()) {
        Some(d) => (d, false),
        None => (build_default(paths), true),
    };
    if crate::edit::ops::effects::lift_always_on_spotlight(&mut doc) || fresh { let _ = doc.save(&paths.edit()); }
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
    let typing = crate::events::track::typing::TypingLog::load(&paths.typing()).ms;

    // Raw regions: auto click-zoom then manual hold-zoom (same order as the exporter;
    // re-anchoring into the screen panel is deferred to render-time, as today).
    let mut raw = if settings.zoom.enabled {
        crate::export::camera::autozoom::generate(&log.events, &log.screen, &cfg, &typing, settings.zoom.smart_hold)
    } else {
        Vec::new()
    };
    raw.extend(crate::export::camera::manual::from_actions(&actions, &log.events, &log.screen, &cfg));

    // Auto/manual zoom regions come out in EVENT time (click/press timestamps); the editor
    // timeline is OUTPUT time (0 = first video frame). Shift zooms onto the output clock so a
    // seeded zoom lands where its pill sits - the same `out = et + events_ms - video_start`
    // conversion clicks use - and so edited/added zooms (already output-time) stay consistent.
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let video_start = tl.frames.first().copied().unwrap_or(0);
    let dur_ms = (tl.frames.last().copied().unwrap_or(video_start).max(video_start + 1) - video_start) as u32;
    let shift = tl.events_ms as i64 - video_start as i64;
    for r in &mut raw {
        r.start_ms = (r.start_ms as i64 + shift).max(0) as u32;
        r.end_ms = (r.end_ms as i64 + shift).max(0) as u32;
    }
    EditDoc {
        version: 1,
        trim: Trim { in_ms: 0, out_ms: dur_ms },
        cuts: vec![],
        zooms: zooms_from_regions(&raw),
        speed: vec![],
        layout: layout_from_actions(&actions, dur_ms),
        effects: spotlight_effects(&actions, dur_ms),
        camera_moves: vec![],
        aspect: crate::export::types::Aspect::default(),
        settings,
    }
}

/// The recording's TRUE full duration (ms) - `video_end - video_start` from the real capture
/// timeline, independent of any user `trim.out_ms` selection. Preview helpers that need "how
/// long is the whole clip" (proxy re-timing in `ensure_proxy`, filmstrip thumbnail spacing in
/// `ensure_thumbs`) call this rather than reading `trim.out_ms` - which, once a user actually
/// trims, no longer means the recording's length. Returns 0 if `events.json` cannot be loaded.
pub fn true_duration_ms(paths: &ProjectPaths) -> u32 {
    let log = match crate::events::model::EventLog::load(&paths.events()) { Ok(l) => l, Err(_) => return 0 };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let vs = tl.frames.first().copied().unwrap_or(0);
    (tl.frames.last().copied().unwrap_or(vs).max(vs + 1) - vs) as u32
}

/// Recorded spotlight holds as editable Spotlight regions (event-time spans clamped to `[0, dur]`,
/// the base seeded zooms use), so a hotkey-held spotlight is an editable/removable timeline pill.
fn spotlight_effects(actions: &[ActionEvent], dur: u32) -> Vec<EffectRegion> {
    crate::export::fx::hold::hold_spans(actions, dur,
        |k| matches!(k, ActionKind::SpotlightHoldStart), |k| matches!(k, ActionKind::SpotlightHoldEnd))
        .into_iter().enumerate().filter(|(_, (s, e))| *e > 0 && *s < dur)
        .map(|(i, (s, e))| EffectRegion { id: format!("e{i}"), kind: EffectKind::Spotlight, start_ms: s.min(dur), end_ms: e.min(dur), fade_in_ms: 250, fade_out_ms: 250, mode: None, dim: None, radius: None, feather: None, layer: 0 })
        .collect()
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;
