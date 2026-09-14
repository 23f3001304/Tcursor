// Tests for edit::seed, split out so seed.rs stays under the size limit.
use super::*;
use crate::actions::model::{ActionEvent, ActionKind, ActionLog, LayoutId};
use crate::events::model::{EventKind, EventLog, MouseEvent, ScreenInfo};
use crate::export::types::{Easing, FramePoint, ZoomRegion};
use crate::session::sync::SyncLog;

/// `sync.json` frame-0 time. With `events_ms = 0` the event clock LEADS the output clock by this
/// much - the ~0.8 s offset real recordings show - so `shift = events_ms - video_start = -800`.
const VIDEO_START: u64 = 800;

/// A minimal on-disk recording: 101 frames 50 ms apart from `VIDEO_START` (a 5000 ms clip). No
/// `settings.json` (defaults apply) and no `video.mp4` - `build_timeline` reads `sync.json`, so
/// nothing here shells out to ffprobe. The single Move event keeps auto-zoom from firing, so the
/// seeded effect/layout spans are the only thing under test.
fn fixture(name: &str, actions: Vec<ActionEvent>) -> ProjectPaths {
    let paths = ProjectPaths { folder: std::env::temp_dir().join(format!("tcursor_seed_{name}")) };
    let _ = std::fs::remove_dir_all(&paths.folder);
    paths.ensure().unwrap();
    let log = EventLog { started_unix_ms: 0, screen: ScreenInfo { w: 1920, h: 1080, origin_x: 0, origin_y: 0 },
        events: vec![MouseEvent { t: 0, kind: EventKind::Move, x: 10, y: 10, button: None }] };
    log.save(&paths.events()).unwrap();
    ActionLog { actions }.save(&paths.actions()).unwrap();
    SyncLog { frames: (0..=100).map(|k| VIDEO_START + k * 50).collect(), events_ms: 0, mic_ms: None, system_ms: None, ..Default::default() }
        .save(&paths.sync()).unwrap();
    paths
}

fn effect(id: &str, start_ms: u32, end_ms: u32) -> EffectRegion {
    EffectRegion { id: id.into(), kind: EffectKind::Spotlight, start_ms, end_ms, fade_in_ms: 250,
        fade_out_ms: 250, mode: None, dim: None, radius: None, feather: None, layer: 0 }
}
fn seg(id: &str, start_ms: u32, end_ms: u32) -> LayoutSeg {
    LayoutSeg { id: id.into(), start_ms, end_ms, layout: "screen".into(), transition_ms: 350, easing: "smooth".into(),
        transition_out_ms: 0, easing_out: "smooth".into(), arrangement: None }
}
fn spans(doc: &EditDoc) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    (doc.effects.iter().map(|e| (e.start_ms, e.end_ms)).collect(),
     doc.layout.iter().map(|s| (s.start_ms, s.end_ms)).collect())
}

fn region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
    ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: 350, zoom_out_ms: 450,
        target_scale: 2.2, anchor: FramePoint { x, y }, easing: Easing::Smooth, layer: 0, cam_action: None,
        follow_cursor: false }
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
fn fields_map_faithfully_and_the_zoom_follows_the_cursor() {
    let zs = zooms_from_regions(&[region(120, 880, 400, 300)]);
    let z = &zs[0];
    assert_eq!(z.start_ms, 120);
    assert_eq!(z.end_ms, 880);
    assert_eq!(z.scale, 2.2);
    assert_eq!(z.easing, "smooth");
    // The click anchor is where the cursor is at the click, so following it starts in the same
    // place a pinned target would and then tracks the hand (owner ruling 2026-09-14).
    assert_eq!(z.target, ZoomTarget::Cursor);
}

#[test]
fn easing_names_map() {
    assert_eq!(easing_str(Easing::Smooth), "smooth");
    assert_eq!(easing_str(Easing::Linear), "linear");
    assert_eq!(easing_str(Easing::Spring { stiffness: 1.0, damping: 1.0, mass: 1.0 }), "spring(1.000,1.000,1.000)");
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

#[test]
fn action_timestamps_shift_onto_the_output_clock_and_saturate_at_zero() {
    let acts = vec![
        ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Camera) },
        ActionEvent { t: 100, kind: ActionKind::SpotlightHoldStart },
    ];
    let out = actions_on_output_clock(&acts, -800);
    assert_eq!(out[0].t, 1200);
    assert_eq!(out[0].kind, acts[0].kind, "only the timestamp moves");
    assert_eq!(out[1].t, 0, "an action before the first video frame clamps to 0, never wraps");
}

/// The whole point of the task: a seeded doc's effect + layout spans must come out on the OUTPUT
/// clock (like its zooms), not the event clock the action log is recorded on.
#[test]
fn seeded_effects_and_layout_are_on_the_output_clock() {
    let paths = fixture("regions_output_clock", vec![
        ActionEvent { t: 2000, kind: ActionKind::SetLayout(LayoutId::Camera) },
        ActionEvent { t: 2000, kind: ActionKind::SpotlightHoldStart },
        ActionEvent { t: 3000, kind: ActionKind::SpotlightHoldEnd },
    ]);
    let doc = build_default(&paths);
    assert_eq!(doc.version, DOC_VERSION, "a freshly seeded doc is written at the current version");
    assert_eq!(doc.trim.out_ms, 5000);
    assert_eq!(doc.clip_ms, 5000, "clip_ms seeds to the true clip duration, same as trim.out_ms");
    let (effects, layout) = spans(&doc);
    assert_eq!(effects, vec![(1200, 2200)], "the 2000..3000 hold fires 800 ms earlier in output time");
    assert_eq!(layout, vec![(0, 1200), (1200, 5000)], "layout switches shift too, still tiling the clip");
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// v1 docs stored effects/layout in event time; loading one must move them onto the output clock
/// exactly once and stamp the doc v2 so a second load is a no-op.
#[test]
fn v1_docs_migrate_their_regions_once() {
    let paths = fixture("migrate_v1", vec![]);
    EditDoc { version: 1, trim: Trim { in_ms: 0, out_ms: 5000 },
        effects: vec![effect("e0", 2000, 3000), effect("e1", 100, 400)],
        layout: vec![seg("l0", 0, 2000), seg("l1", 2000, 5000)],
        ..Default::default() }.save(&paths.edit()).unwrap();

    let doc = load_or_seed(&paths);
    assert_eq!(doc.version, DOC_VERSION);
    assert_eq!(spans(&doc), (vec![(1200, 2200), (0, 0)], vec![(0, 1200), (1200, 4200)]));
    assert_eq!(doc.clip_ms, 5000, "migrate backfills clip_ms from the true recording length too");

    let again = load_or_seed(&paths); // now v2 on disk - must not shift a second time
    assert_eq!(spans(&again), spans(&doc), "migration must be idempotent");
    assert_eq!(again.version, DOC_VERSION);
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// No `events.json`/`sync.json` -> the shift is unknowable. The doc must still be stamped v2 (so it
/// is never re-migrated later, when a timeline WOULD resolve) with its regions left untouched.
#[test]
fn a_v1_doc_without_a_timeline_becomes_v2_unshifted() {
    let paths = ProjectPaths { folder: std::env::temp_dir().join("tcursor_seed_migrate_no_timeline") };
    let _ = std::fs::remove_dir_all(&paths.folder);
    paths.ensure().unwrap();
    EditDoc { version: 1, trim: Trim { in_ms: 0, out_ms: 5000 },
        effects: vec![effect("e0", 2000, 3000)], layout: vec![seg("l0", 0, 5000)],
        ..Default::default() }.save(&paths.edit()).unwrap();
    let doc = load_or_seed(&paths);
    assert_eq!(doc.version, DOC_VERSION);
    assert_eq!(spans(&doc), (vec![(2000, 3000)], vec![(0, 5000)]));
    assert_eq!(doc.clip_ms, 0, "clip_ms stays unknown too - same reason as the regions");
    let _ = std::fs::remove_dir_all(&paths.folder);
}

/// A doc already at `DOC_VERSION` but written before `clip_ms` existed (defaults to 0 on load)
/// must still get backfilled - the backfill is a field fix, not gated by the version-bump `if`.
#[test]
fn clip_ms_backfills_on_an_already_current_version_doc_too() {
    let paths = fixture("clip_ms_backfill_v2", vec![]);
    EditDoc { version: DOC_VERSION, trim: Trim { in_ms: 0, out_ms: 5000 }, ..Default::default() }
        .save(&paths.edit()).unwrap();
    let doc = load_or_seed(&paths);
    assert_eq!(doc.clip_ms, 5000);
    let _ = std::fs::remove_dir_all(&paths.folder);
}
