use super::*;
use crate::edit::model::*;
use crate::export::remap::TimeMap;

fn map() -> TimeMap { crate::export::remap::tests::fixture() }

fn spot(id: &str, start_ms: u32, end_ms: u32) -> EffectRegion {
    EffectRegion { id: id.into(), kind: EffectKind::Spotlight, start_ms, end_ms, fade_in_ms: 250, fade_out_ms: 250,
        mode: None, dim: None, radius: None, feather: None, layer: 0 }
}

#[test]
fn regions_move_to_the_output_clock_and_keep_their_durations() {
    let mut d = EditDoc::default();
    d.zooms.push(Zoom { id: "z0".into(), start_ms: 2200, end_ms: 3200, target: ZoomTarget::Cursor, scale: 2.0, easing: "smooth".into(),
        zoom_in_ms: 350, zoom_out_ms: 450, layer: 0, cam_action: None });
    let r = remap_doc(&d, &map());
    assert_eq!((r.zooms[0].start_ms, r.zooms[0].end_ms), (700, 1350));
    assert_eq!((r.zooms[0].zoom_in_ms, r.zooms[0].zoom_out_ms), (350, 450));
    assert_eq!(r.zooms[0].id, "z0");
}

#[test]
fn a_region_entirely_inside_a_cut_is_dropped_and_one_straddling_it_shrinks() {
    let mut d = EditDoc::default();
    d.effects.push(spot("e0", 1100, 1900));
    d.effects.push(spot("e1", 800, 2200));
    let r = remap_doc(&d, &map());
    assert_eq!(r.effects.len(), 1);
    assert_eq!((r.effects[0].id.as_str(), r.effects[0].start_ms, r.effects[0].end_ms), ("e1", 300, 700));
}

#[test]
fn camera_moves_map_their_time_and_the_consumed_fields_are_cleared() {
    let mut d = EditDoc::default();
    d.trim = Trim { in_ms: 500, out_ms: 9000 };
    d.cuts.push(Cut { id: "c0".into(), start_ms: 1000, end_ms: 2000 });
    d.speed.push(Speed { id: "s0".into(), start_ms: 2500, end_ms: 3500, factor: 2.0 });
    d.camera_moves.push(CameraMove { id: "m0".into(), t_ms: 1500, x: 0.5, y: 0.5, size: 0.3, easing: "smooth".into() });
    let r = remap_doc(&d, &map());
    assert_eq!(r.camera_moves[0].t_ms, 500);
    assert_eq!(r.trim, Trim::default());
    assert!(r.cuts.is_empty() && r.speed.is_empty());
    assert_eq!(r.clip_ms, 8500);
}

#[test]
fn a_plain_map_is_the_identity_on_regions() {
    let mut d = EditDoc::default();
    d.layout.push(LayoutSeg { id: "l0".into(), start_ms: 100, end_ms: 900, layout: "camera".into(), transition_ms: 350, easing: "smooth".into(),
        transition_out_ms: 0, easing_out: "smooth".into(), arrangement: None });
    let r = remap_doc(&d, &TimeMap::identity(10_000));
    assert_eq!(r.layout, d.layout);
}
