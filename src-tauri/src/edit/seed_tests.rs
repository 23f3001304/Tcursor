// Tests for edit::seed, split out so seed.rs stays under the size limit.
use super::*;
use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
use crate::export::types::{Easing, FramePoint, ZoomRegion};

fn region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
    ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: 350, zoom_out_ms: 450,
        target_scale: 2.2, anchor: FramePoint { x, y }, easing: Easing::Smooth, layer: 0, cam_action: None }
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
