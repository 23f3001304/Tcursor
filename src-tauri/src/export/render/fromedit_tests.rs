use super::*;
use crate::edit::model::EditDoc;
use crate::edit::seed::{layout_from_actions, zooms_from_regions};
use crate::export::types::{Easing, FramePoint, ZoomRegion};

fn cfg_region(start: u32, end: u32, x: i32, y: i32) -> ZoomRegion {
    let cfg = crate::settings::model::Settings::default()
        .zoom
        .to_zoom_config();
    ZoomRegion {
        start_ms: start,
        end_ms: end,
        zoom_in_ms: cfg.zoom_in_ms,
        zoom_out_ms: cfg.zoom_out_ms,
        target_scale: cfg.target_scale,
        anchor: FramePoint { x, y },
        easing: cfg.easing,
        easing_out: cfg.easing,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    }
}

fn eq_region(a: &ZoomRegion, b: &ZoomRegion) -> bool {
    a.start_ms == b.start_ms
        && a.end_ms == b.end_ms
        && a.zoom_in_ms == b.zoom_in_ms
        && a.zoom_out_ms == b.zoom_out_ms
        && a.target_scale == b.target_scale
        && a.layer == b.layer
        && a.anchor == b.anchor
        && format!("{:?}", a.easing) == format!("{:?}", b.easing)
        && format!("{:?}", a.easing_out) == format!("{:?}", b.easing_out)
        && a.cam_action == b.cam_action
        && a.follow_cursor == b.follow_cursor
}

#[test]
fn an_absent_easing_out_falls_back_to_the_zooms_own_easing() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let base = Zoom {
        id: "z0".into(),
        start_ms: 0,
        end_ms: 1000,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "linear".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 0,
        cam_action: None,
        smart_typing: false,
        easing_out: None,
    };
    let doc = EditDoc {
        zooms: vec![base.clone()],
        ..Default::default()
    };
    let r = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(
        format!("{:?}", r[0].easing_out),
        format!("{:?}", Easing::Linear)
    );
    let split = EditDoc {
        zooms: vec![Zoom {
            easing_out: Some("ease_in".into()),
            ..base
        }],
        ..Default::default()
    };
    let r = regions_from_doc(&split, 1920, 1080);
    assert_eq!(
        format!("{:?}", r[0].easing),
        format!("{:?}", Easing::Linear)
    );
    assert_eq!(
        format!("{:?}", r[0].easing_out),
        format!("{:?}", Easing::EaseIn)
    );
}

#[test]
fn regions_round_trip_through_edit_doc() {
    let orig = vec![
        cfg_region(0, 2650, 100, 200),
        cfg_region(5000, 7650, 1500, 900),
        cfg_region(9000, 9450, 0, 0),
    ];
    let doc = EditDoc {
        zooms: zooms_from_regions(&orig),
        ..Default::default()
    };
    let rebuilt = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(rebuilt.len(), orig.len());
    for (i, (a, b)) in rebuilt.iter().zip(orig.iter()).enumerate() {
        let want = ZoomRegion {
            follow_cursor: true,
            anchor: a.anchor,
            ..*b
        };
        assert!(
            eq_region(a, &want),
            "region {} drifted: {:?} != {:?}",
            i,
            a,
            want
        );
        assert!(a.follow_cursor, "a seeded auto zoom follows the cursor");
    }
}

#[test]
fn layer_round_trips_through_edit_doc() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let mut doc = EditDoc::default();
    doc.zooms.push(Zoom {
        id: "z0".into(),
        start_ms: 0,
        end_ms: 1000,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "smooth".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 3,
        cam_action: None,
        smart_typing: false,
        easing_out: None,
    });
    let regs = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(regs[0].layer, 3);
}

#[test]
fn cam_action_round_trips_through_edit_doc() {
    use crate::edit::model::{Zoom, ZoomTarget};
    use crate::settings::model::CamZoomAction;
    let mut doc = EditDoc::default();
    doc.zooms.push(Zoom {
        id: "z0".into(),
        start_ms: 0,
        end_ms: 1000,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "smooth".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 0,
        cam_action: Some(CamZoomAction::Hide),
        smart_typing: false,
        easing_out: None,
    });
    let regs = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(regs[0].cam_action, Some(CamZoomAction::Hide));
    let back = crate::edit::seed::zooms_from_regions(&regs);
    assert_eq!(
        back[0].cam_action,
        Some(CamZoomAction::Hide),
        "seed inverse must carry it too"
    );
}

#[test]
fn empty_layout_signals_fallback() {
    assert!(layout_segs_from_doc(&EditDoc::default()).is_none());
}

#[test]
fn layout_segs_round_trip() {
    use crate::actions::model::{ActionEvent, ActionKind, LayoutId};
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
    let doc = EditDoc {
        layout: layout_from_actions(&acts, 8000),
        ..Default::default()
    };
    let rebuilt = layout_segs_from_doc(&doc).unwrap();
    assert_eq!(
        rebuilt[0],
        ActionEvent {
            t: 0,
            kind: ActionKind::SetLayout(LayoutId::Screen)
        }
    );
    assert_eq!(rebuilt[1], acts[0]);
    assert_eq!(rebuilt[2], acts[1]);
}

#[path = "fromedit_zoom_tests.rs"]
mod zoom_tests;
