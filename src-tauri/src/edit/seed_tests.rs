use super::*;
use crate::actions::model::{ActionEvent, ActionKind, ActionLog, LayoutId};
use crate::events::model::{EventKind, EventLog, MouseEvent, ScreenInfo};
use crate::export::types::{Easing, FramePoint, ZoomRegion};
use crate::session::sync::SyncLog;

const VIDEO_START: u64 = 800;

fn fixture(name: &str, actions: Vec<ActionEvent>) -> ProjectPaths {
    let paths = ProjectPaths {
        folder: std::env::temp_dir().join(format!("tcursor_seed_{name}")),
    };
    let _ = std::fs::remove_dir_all(&paths.folder);
    paths.ensure().unwrap();
    let log = EventLog {
        started_unix_ms: 0,
        screen: ScreenInfo {
            w: 1920,
            h: 1080,
            origin_x: 0,
            origin_y: 0,
        },
        events: vec![MouseEvent {
            t: 0,
            kind: EventKind::Move,
            x: 10,
            y: 10,
            button: None,
        }],
    };
    log.save(&paths.events()).unwrap();
    ActionLog { actions }.save(&paths.actions()).unwrap();
    SyncLog {
        frames: (0..=100).map(|k| VIDEO_START + k * 50).collect(),
        events_ms: 0,
        mic_ms: None,
        system_ms: None,
        ..Default::default()
    }
    .save(&paths.sync())
    .unwrap();
    paths
}

fn effect(id: &str, start_ms: u32, end_ms: u32) -> EffectRegion {
    EffectRegion {
        id: id.into(),
        kind: EffectKind::Spotlight,
        start_ms,
        end_ms,
        fade_in_ms: 250,
        fade_out_ms: 250,
        mode: None,
        dim: None,
        radius: None,
        feather: None,
        layer: 0,
    }
}
fn seg(id: &str, start_ms: u32, end_ms: u32) -> LayoutSeg {
    LayoutSeg {
        id: id.into(),
        start_ms,
        end_ms,
        layout: "screen".into(),
        transition_ms: 350,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    }
}
fn spans(doc: &EditDoc) -> (Vec<(u32, u32)>, Vec<(u32, u32)>) {
    (
        doc.effects.iter().map(|e| (e.start_ms, e.end_ms)).collect(),
        doc.layout.iter().map(|s| (s.start_ms, s.end_ms)).collect(),
    )
}

fn region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
    ZoomRegion {
        start_ms: start,
        end_ms: end,
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        target_scale: 2.2,
        anchor: FramePoint { x, y },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    }
}

#[test]
fn ids_are_deterministic_and_count_matches() {
    let regs = vec![
        region(0, 100, 10, 20),
        region(500, 900, 30, 40),
        region(2000, 2500, 5, 6),
    ];
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
    assert_eq!(z.target, ZoomTarget::Cursor);
}

#[test]
fn easing_names_map() {
    assert_eq!(easing_str(Easing::Smooth), "smooth");
    assert_eq!(easing_str(Easing::Linear), "linear");
    assert_eq!(
        easing_str(Easing::Spring {
            stiffness: 1.0,
            damping: 1.0,
            mass: 1.0
        }),
        "spring(1.000,1.000,1.000)"
    );
}

#[test]
fn a_keys_curve_round_trips_through_the_wire() {
    let canon =
        "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)";
    let e = crate::export::render::fromedit::easing_from(canon, Easing::Smooth);
    assert!(matches!(e, Easing::Keys(_)));
    assert_eq!(easing_str(e), canon);
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
        ActionEvent {
            t: 1000,
            kind: ActionKind::SetLayout(LayoutId::Camera),
        },
        ActionEvent {
            t: 3000,
            kind: ActionKind::SetLayout(LayoutId::ScreenOnly),
        },
    ];
    let segs = layout_from_actions(&acts, 8000);
    assert_eq!(segs.len(), 3);
    assert_eq!(
        (segs[0].start_ms, segs[0].end_ms, segs[0].layout.as_str()),
        (0, 1000, "screen")
    );
    assert_eq!(
        (segs[1].start_ms, segs[1].end_ms, segs[1].layout.as_str()),
        (1000, 3000, "camera")
    );
    assert_eq!(
        (segs[2].start_ms, segs[2].end_ms, segs[2].layout.as_str()),
        (3000, 8000, "screen_only")
    );
}

#[test]
fn action_timestamps_shift_onto_the_output_clock_and_saturate_at_zero() {
    let acts = vec![
        ActionEvent {
            t: 2000,
            kind: ActionKind::SetLayout(LayoutId::Camera),
        },
        ActionEvent {
            t: 100,
            kind: ActionKind::SpotlightHoldStart,
        },
    ];
    let out = actions_on_output_clock(&acts, -800);
    assert_eq!(out[0].t, 1200);
    assert_eq!(out[0].kind, acts[0].kind, "only the timestamp moves");
    assert_eq!(
        out[1].t, 0,
        "an action before the first video frame clamps to 0, never wraps"
    );
}

#[test]
fn seeded_effects_and_layout_are_on_the_output_clock() {
    let paths = fixture(
        "regions_output_clock",
        vec![
            ActionEvent {
                t: 2000,
                kind: ActionKind::SetLayout(LayoutId::Camera),
            },
            ActionEvent {
                t: 2000,
                kind: ActionKind::SpotlightHoldStart,
            },
            ActionEvent {
                t: 3000,
                kind: ActionKind::SpotlightHoldEnd,
            },
        ],
    );
    let doc = build_default(&paths);
    assert_eq!(
        doc.version, DOC_VERSION,
        "a freshly seeded doc is written at the current version"
    );
    assert_eq!(doc.trim.out_ms, 5000);
    assert_eq!(
        doc.clip_ms, 5000,
        "clip_ms seeds to the true clip duration, same as trim.out_ms"
    );
    let (effects, layout) = spans(&doc);
    assert_eq!(
        effects,
        vec![(1200, 2200)],
        "the 2000..3000 hold fires 800 ms earlier in output time"
    );
    assert_eq!(
        layout,
        vec![(0, 1200), (1200, 5000)],
        "layout switches shift too, still tiling the clip"
    );
    let _ = std::fs::remove_dir_all(&paths.folder);
}

#[path = "seed_migrate_tests.rs"]
mod migrate_tests;
