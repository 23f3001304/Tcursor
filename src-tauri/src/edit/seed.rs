use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::edit::model::{
    EditDoc, EffectKind, EffectRegion, LayoutSeg, Trim, Zoom, ZoomTarget, DOC_VERSION,
};
use crate::export::types::{Easing, ZoomRegion};
use crate::session::paths::ProjectPaths;

pub(crate) use crate::edit::migrate::output_shift;
pub use crate::edit::migrate::true_duration_ms;

fn easing_str(e: Easing) -> String {
    match e {
        Easing::Smooth => "smooth".into(),
        Easing::Linear => "linear".into(),
        Easing::Spring {
            stiffness,
            damping,
            mass,
        } => crate::export::spring::format_spring(stiffness, damping, mass),
        Easing::EaseIn => "ease_in".into(),
        Easing::EaseOut => "ease_out".into(),
        Easing::EaseInOut => "ease_in_out".into(),
        Easing::Cubic { x1, y1, x2, y2 } => crate::export::cubic::format_cubic(x1, y1, x2, y2),
        Easing::Keys(ref k) => crate::export::keys::format_keys(k),
    }
}

pub fn zooms_from_regions(regions: &[ZoomRegion]) -> Vec<Zoom> {
    regions
        .iter()
        .enumerate()
        .map(|(i, r)| Zoom {
            id: format!("z{}", i),
            start_ms: r.start_ms,
            end_ms: r.end_ms,
            target: ZoomTarget::Cursor,
            scale: r.target_scale,
            easing: easing_str(r.easing),
            zoom_in_ms: r.zoom_in_ms,
            zoom_out_ms: r.zoom_out_ms,
            layer: r.layer,
            cam_action: r.cam_action,
            smart_typing: false,
            easing_out: (r.easing_out != r.easing).then(|| easing_str(r.easing_out)),
        })
        .collect()
}

fn layout_name(id: LayoutId) -> String {
    serde_json::to_value(id)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "screen".to_string())
}

pub fn layout_from_actions(actions: &[ActionEvent], dur_ms: u32) -> Vec<LayoutSeg> {
    let mut switches: Vec<(u32, LayoutId)> = vec![(0, LayoutId::Screen)];
    for a in actions {
        if let ActionKind::SetLayout(id) = a.kind {
            switches.push((a.t, id));
        }
    }
    let mut segs = Vec::with_capacity(switches.len());
    for (i, &(start, id)) in switches.iter().enumerate() {
        let end = switches
            .get(i + 1)
            .map(|&(s, _)| s)
            .unwrap_or(dur_ms)
            .max(start);
        segs.push(LayoutSeg {
            id: format!("l{}", i),
            start_ms: start,
            end_ms: end,
            layout: layout_name(id),
            transition_ms: 350,
            easing: "smooth".into(),
            transition_out_ms: 0,
            easing_out: "smooth".into(),
            arrangement: None,
        });
    }
    segs
}

pub fn load_or_seed(paths: &ProjectPaths) -> EditDoc {
    let unlocked = EditDoc::load(&paths.edit());
    if let Some(doc) = &unlocked {
        if !needs_seed_write(doc) {
            return unlocked.unwrap();
        }
    }
    let (precomputed_default, shift, true_dur) = derive_seed_inputs(paths, &unlocked);
    let lock = crate::edit::lock::doc_lock(paths);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    load_or_seed_locked(paths, precomputed_default, shift, true_dur)
}

pub(crate) fn load_or_seed_locked(
    paths: &ProjectPaths,
    precomputed_default: Option<EditDoc>,
    shift: i64,
    true_dur: u32,
) -> EditDoc {
    let (mut doc, fresh) = match EditDoc::load(&paths.edit()) {
        Some(d) => (d, false),
        None => (
            precomputed_default.unwrap_or_else(|| crate::edit::seed::build_default(paths)),
            true,
        ),
    };
    let migrated = crate::edit::migrate::migrate(&mut doc, shift, true_dur);
    if crate::edit::ops::effects::lift_always_on_spotlight(&mut doc) || fresh || migrated {
        let _ = doc.save(&paths.edit());
    }
    doc
}

pub(crate) fn derive_seed_inputs(
    paths: &ProjectPaths,
    unlocked: &Option<EditDoc>,
) -> (Option<EditDoc>, i64, u32) {
    let needs_shift = unlocked.as_ref().map_or(false, |d| d.version < 2);
    let needs_true_dur = unlocked.as_ref().map_or(false, |d| d.clip_ms == 0);
    let precomputed_default = if unlocked.is_none() {
        Some(crate::edit::seed::build_default(paths))
    } else {
        None
    };
    let shift = if needs_shift {
        crate::edit::seed::output_shift(paths)
    } else {
        0
    };
    let true_dur = if needs_true_dur {
        crate::edit::seed::true_duration_ms(paths)
    } else {
        0
    };
    (precomputed_default, shift, true_dur)
}

fn needs_seed_write(doc: &EditDoc) -> bool {
    doc.version < DOC_VERSION
        || doc.clip_ms == 0
        || (doc.settings.clickfx.spotlight
            && !doc
                .effects
                .iter()
                .any(|e| matches!(e.kind, EffectKind::Spotlight)))
}

pub fn actions_on_output_clock(actions: &[ActionEvent], shift: i64) -> Vec<ActionEvent> {
    actions
        .iter()
        .map(|a| ActionEvent {
            t: (a.t as i64 + shift).max(0) as u32,
            kind: a.kind,
        })
        .collect()
}

pub(crate) fn build_default(paths: &ProjectPaths) -> EditDoc {
    let settings = crate::settings::store::record_snapshot(paths);
    let cfg = settings.zoom.to_zoom_config();
    let log = match crate::events::model::EventLog::load(&paths.events()) {
        Ok(l) => l,
        Err(_) => {
            return EditDoc {
                settings,
                ..Default::default()
            }
        }
    };
    let actions = crate::actions::model::ActionLog::load(&paths.actions())
        .map(|a| a.actions)
        .unwrap_or_default();
    let typing = crate::events::track::typing::TypingLog::load(&paths.typing()).ms;

    let mut raw = if settings.zoom.enabled {
        crate::export::camera::autozoom::generate(
            &log.events,
            &log.screen,
            &cfg,
            &typing,
            settings.zoom.smart_hold,
        )
    } else {
        Vec::new()
    };
    raw.extend(crate::export::camera::manual::from_actions(
        &actions,
        &log.events,
        &log.screen,
        &cfg,
    ));

    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let video_start = tl.frames.first().copied().unwrap_or(0);
    let dur_ms = (tl
        .frames
        .last()
        .copied()
        .unwrap_or(video_start)
        .max(video_start + 1)
        - video_start) as u32;
    let shift = tl.events_ms as i64 - video_start as i64;
    for r in &mut raw {
        r.start_ms = (r.start_ms as i64 + shift).max(0) as u32;
        r.end_ms = (r.end_ms as i64 + shift).max(0) as u32;
    }
    let out_actions = actions_on_output_clock(&actions, shift);
    EditDoc {
        version: DOC_VERSION,
        trim: Trim {
            in_ms: 0,
            out_ms: dur_ms,
        },
        clip_ms: dur_ms,
        cuts: vec![],
        zooms: zooms_from_regions(&raw),
        speed: vec![],
        layout: layout_from_actions(&out_actions, dur_ms),
        effects: spotlight_effects(&out_actions, dur_ms),
        camera_moves: vec![],
        aspect: crate::export::types::Aspect::default(),
        settings,
        captions: vec![],
        texts: vec![],
        clips: vec![],
    }
}

fn spotlight_effects(actions: &[ActionEvent], dur: u32) -> Vec<EffectRegion> {
    crate::export::fx::hold::hold_spans(
        actions,
        dur,
        |k| matches!(k, ActionKind::SpotlightHoldStart),
        |k| matches!(k, ActionKind::SpotlightHoldEnd),
    )
    .into_iter()
    .enumerate()
    .filter(|(_, (s, e))| *e > 0 && *s < dur)
    .map(|(i, (s, e))| EffectRegion {
        id: format!("e{i}"),
        kind: EffectKind::Spotlight,
        start_ms: s.min(dur),
        end_ms: e.min(dur),
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
        rect: None,
        strength: None,
        roundness: None,
    })
    .collect()
}

#[cfg(test)]
#[path = "seed_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "seed_lock_tests.rs"]
mod seed_lock_tests;
