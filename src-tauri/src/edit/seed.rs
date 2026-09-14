// Build a default EditDoc from a recording, identical to today's export inputs.
// load_or_seed returns an existing edit.json or seeds one from the same auto-zoom,
// manual-zoom, layout-track and settings the exporter derives, then writes it.
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{EditDoc, EffectKind, EffectRegion, LayoutSeg, Trim, Zoom, ZoomTarget, DOC_VERSION};
use crate::export::types::{Easing, ZoomRegion};
use crate::session::paths::ProjectPaths;

// Doc-version migrations + the shared event/output clock shift live in their own file (seed.rs
// was at the 200-line budget); re-exported here so callers keep using the `seed::` path.
pub use crate::edit::migrate::true_duration_ms;
pub(crate) use crate::edit::migrate::output_shift;

/// Stable lowercase name for an easing curve (mirrors the export `Easing` enum). `Spring` and
/// `Cubic` carry parameters, so they emit their own wire form rather than a bare word - both
/// round-trip exactly through `valid_easing` + `easing_from`.
fn easing_str(e: Easing) -> String {
    match e {
        Easing::Smooth => "smooth".into(),
        Easing::Linear => "linear".into(),
        Easing::Spring { stiffness, damping, mass } =>
            crate::export::spring::format_spring(stiffness, damping, mass),
        Easing::EaseIn => "ease_in".into(),
        Easing::EaseOut => "ease_out".into(),
        Easing::EaseInOut => "ease_in_out".into(),
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::format_cubic(x1, y1, x2, y2),
    }
}

/// PURE: one `Zoom` per region, ids `z0, z1, ...`. Every seeded zoom FOLLOWS the cursor
/// (`ZoomTarget::Cursor`): an auto zoom fires on a click, and at that instant the cursor
/// IS the region's click anchor, so the zoom-in lands where the old `Fixed` anchor did and
/// then tracks the hand instead of staying pinned there (owner ruling 2026-09-14 - a fresh
/// recording's zooms showed as "Region" in the inspector). The anchor is dropped; a user
/// who wants a pinned aim switches that zoom to Region. Output length equals input length.
pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom> {
    regions.iter().enumerate().map(|(i, r)| Zoom {
        id: format!("z{}", i), start_ms: r.start_ms, end_ms: r.end_ms,
        target: ZoomTarget::Cursor,
        scale: r.target_scale, easing: easing_str(r.easing),
        zoom_in_ms: r.zoom_in_ms, zoom_out_ms: r.zoom_out_ms, layer: r.layer,
        cam_action: r.cam_action, smart_typing: false,
    }).collect()
}

/// Serde wire name for a `LayoutId` (e.g. `screen`, `screen_only`).
fn layout_name(id: LayoutId) -> String {
    serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "screen".to_string())
}

/// PURE: one `LayoutSeg` per active span in the `SetLayout` track (mirrors `LayoutTrack`: always
/// starts at `(0, Screen)`), each span running to the next switch and the last to `dur_ms`. Empty
/// track -> a single `screen` span [0, dur]. `actions` and `dur_ms` share one clock (the seed
/// passes both on the OUTPUT clock).
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
            transition_ms: 350, easing: "smooth".into(), transition_out_ms: 0, easing_out: "smooth".into(), arrangement: None });
    }
    segs
}

// `load_or_seed` - the self-locking public entry point (H3, bug-sweep-2 Task 7 round 2) - lives
// in `seed_lock` alongside the lower-level pieces (`load_or_seed_locked`, `derive_seed_inputs`)
// that make it correct without holding the per-folder lock across a `build_timeline`/ffprobe
// call; re-exported here so callers keep using the `seed::` path. See `seed_lock.rs`'s module
// doc for the full design (two-phase locking, and why `edit::commands::apply_edit_op` calls
// `load_or_seed_locked` directly instead of this function).
pub use crate::edit::seed_lock::load_or_seed;

/// PURE: recorded actions with every timestamp moved onto the output clock by `shift` (saturating
/// at 0), kinds untouched. Both region seeders (`layout_from_actions`, `spotlight_effects`) and the
/// renderer's recorded-layout fallback build from the SAME shifted log, so they cannot disagree.
pub fn actions_on_output_clock(actions: &[ActionEvent], shift: i64) -> Vec<ActionEvent> {
    actions.iter().map(|a| ActionEvent { t: (a.t as i64 + shift).max(0) as u32, kind: a.kind }).collect()
}

/// Construct the default doc. Mirrors `exporter::export`'s input build exactly so a
/// seeded render matches today: settings snapshot -> `ZoomConfig`, auto + manual raw
/// zoom regions, the `SetLayout` track, and `[0, clip duration]` for the trim.
/// `pub(crate)` so `seed_lock` (the fast-path/slow-path locking wrapper) can call it directly.
pub(crate) fn build_default(paths: &ProjectPaths) -> EditDoc {
    let settings = crate::settings::store::record_snapshot(paths);
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

    // Everything the recording hands us is EVENT time (click/press timestamps); every region list
    // in an `EditDoc` is OUTPUT time (0 = first video frame). Shift once here - the same
    // `out = et + events_ms - video_start` conversion clicks use - so seeded regions land where
    // their pills sit and edited/added ones (already output-time) stay consistent.
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let video_start = tl.frames.first().copied().unwrap_or(0);
    let dur_ms = (tl.frames.last().copied().unwrap_or(video_start).max(video_start + 1) - video_start) as u32;
    let shift = tl.events_ms as i64 - video_start as i64;
    for r in &mut raw {
        r.start_ms = (r.start_ms as i64 + shift).max(0) as u32;
        r.end_ms = (r.end_ms as i64 + shift).max(0) as u32;
    }
    // Layout + spotlight regions are derived from the action log, so shift IT rather than the
    // spans: the derived segments then tile `[0, dur_ms]` on the output clock with no tail gap.
    let out_actions = actions_on_output_clock(&actions, shift);
    EditDoc {
        version: DOC_VERSION,
        trim: Trim { in_ms: 0, out_ms: dur_ms },
        clip_ms: dur_ms,
        cuts: vec![],
        zooms: zooms_from_regions(&raw),
        speed: vec![],
        layout: layout_from_actions(&out_actions, dur_ms),
        effects: spotlight_effects(&out_actions, dur_ms),
        camera_moves: vec![],
        aspect: crate::export::types::Aspect::default(),
        settings,
    }
}

/// Recorded spotlight holds as editable Spotlight regions, clamped to `[0, dur]`. `actions` must
/// already be on the output clock (`actions_on_output_clock`), like every other seeded region.
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
