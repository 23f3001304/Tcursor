// Tests for export::render::fromedit, split out so fromedit.rs stays under the size limit.
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
        cam_action: None, follow_cursor: false,
    }
}

fn eq_region(a: &ZoomRegion, b: &ZoomRegion) -> bool {
    a.start_ms == b.start_ms && a.end_ms == b.end_ms && a.zoom_in_ms == b.zoom_in_ms
        && a.zoom_out_ms == b.zoom_out_ms && a.target_scale == b.target_scale && a.layer == b.layer
        && a.anchor == b.anchor && format!("{:?}", a.easing) == format!("{:?}", b.easing)
        && a.cam_action == b.cam_action && a.follow_cursor == b.follow_cursor
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
        scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 3, cam_action: None });
    let regs = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(regs[0].layer, 3);
}

/// A per-zoom `cam_action` survives doc -> regions, AND back out through the seed
/// inverse - so the round-trip stays lossless now that the field exists.
#[test]
fn cam_action_round_trips_through_edit_doc() {
    use crate::edit::model::{Zoom, ZoomTarget};
    use crate::settings::model::CamZoomAction;
    let mut doc = EditDoc::default();
    doc.zooms.push(Zoom { id: "z0".into(), start_ms: 0, end_ms: 1000, target: ZoomTarget::Cursor,
        scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 0,
        cam_action: Some(CamZoomAction::Hide) });
    let regs = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(regs[0].cam_action, Some(CamZoomAction::Hide));
    let back = crate::edit::seed::zooms_from_regions(&regs);
    assert_eq!(back[0].cam_action, Some(CamZoomAction::Hide), "seed inverse must carry it too");
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
            target: ZoomTarget::Cursor, scale: 2.0, easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 0, cam_action: None }],
        ..Default::default()
    };
    let r = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(r[0].anchor, FramePoint { x: 960, y: 540 });
    // ...and `follow_cursor` is what makes that centre fallback inert: `CameraSim` aims the
    // zoom-in ramp at the live cursor instead of reading it.
    assert!(r[0].follow_cursor, "a Cursor target must mark the region as live-cursor-aimed");
    let fixed = EditDoc { zooms: vec![Zoom { target: ZoomTarget::Fixed { x: 100.0, y: 200.0 },
        ..doc.zooms[0].clone() }], ..Default::default() };
    assert!(!regions_from_doc(&fixed, 1920, 1080)[0].follow_cursor, "a Fixed target keeps its stored anchor");
}

/// The Region target the stage reticle writes is a 0..1 screen-content FRACTION, so it must scale
/// into screen pixels; out-of-range values are already pixels and pass through untouched.
#[test]
fn fixed_target_fractions_scale_to_screen_pixels() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let base = Zoom { id: "z0".into(), start_ms: 0, end_ms: 100, target: ZoomTarget::Cursor, scale: 2.0,
        easing: "smooth".into(), zoom_in_ms: 350, zoom_out_ms: 450, layer: 0, cam_action: None };
    let doc_of = |t| EditDoc { zooms: vec![Zoom { target: t, ..base.clone() }], ..Default::default() };
    let frac = regions_from_doc(&doc_of(ZoomTarget::Fixed { x: 0.25, y: 0.75 }), 1920, 1080);
    assert_eq!(frac[0].anchor, FramePoint { x: 480, y: 810 });
    let px = regions_from_doc(&doc_of(ZoomTarget::Fixed { x: 1300.0, y: 40.0 }), 1920, 1080);
    assert_eq!(px[0].anchor, FramePoint { x: 1300, y: 40 });
}

#[test]
fn per_region_durations_flow_into_regions() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let mut doc = EditDoc::default();
    doc.zooms.push(Zoom { id: "z0".into(), start_ms: 0, end_ms: 1000, target: ZoomTarget::Cursor,
        scale: 2.0, easing: "smooth".into(), zoom_in_ms: 120, zoom_out_ms: 640, layer: 0, cam_action: None });
    let regs = regions_from_doc(&doc, 800, 600);
    assert_eq!((regs[0].zoom_in_ms, regs[0].zoom_out_ms), (120, 640));
}

#[test]
fn easing_unknown_falls_back_to_config() {
    let cfg = crate::settings::model::Settings::default().zoom.to_zoom_config();
    assert_eq!(format!("{:?}", easing_from("bogus", cfg.easing)), format!("{:?}", cfg.easing));
    assert_eq!(format!("{:?}", easing_from("smooth", cfg.easing)), format!("{:?}", Easing::Smooth));
    assert_eq!(format!("{:?}", easing_from("linear", cfg.easing)), format!("{:?}", Easing::Linear));
}

/// THE FIX: "spring" used to fall into `_`, fail `parse_cubic`, and return `cfg_easing` (Smooth).
#[test]
fn spring_maps_to_a_real_spring_variant() {
    let cfg = crate::settings::model::Settings::default().zoom.to_zoom_config();
    assert!(matches!(easing_from("spring", cfg.easing), Easing::Spring { .. }));
    // A parameterised spring reconstructs exactly, mass defaulted; junk falls back to the config.
    assert_eq!(easing_from("spring(300,10)", cfg.easing),
        Easing::Spring { stiffness: 300.0, damping: 10.0, mass: 1.0 });
    assert_eq!(easing_from("spring(300,10,2)", cfg.easing),
        Easing::Spring { stiffness: 300.0, damping: 10.0, mass: 2.0 });
    assert_eq!(easing_from("spring(300)", cfg.easing), cfg.easing);
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
