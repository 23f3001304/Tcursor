use super::*;

#[test]
fn empty_zooms_make_no_regions() {
    let doc = EditDoc::default();
    assert!(regions_from_doc(&doc, 1920, 1080).is_empty());
}

#[test]
fn cursor_target_defaults_to_screen_center() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let doc = EditDoc {
        zooms: vec![Zoom {
            id: "z0".into(),
            start_ms: 0,
            end_ms: 100,
            target: ZoomTarget::Cursor,
            scale: 2.0,
            easing: "smooth".into(),
            zoom_in_ms: 350,
            zoom_out_ms: 450,
            layer: 0,
            cam_action: None,
            smart_typing: false,
            easing_out: None,
        }],
        ..Default::default()
    };
    let r = regions_from_doc(&doc, 1920, 1080);
    assert_eq!(r[0].anchor, FramePoint { x: 960, y: 540 });
    assert!(
        r[0].follow_cursor,
        "a Cursor target must mark the region as live-cursor-aimed"
    );
    let fixed = EditDoc {
        zooms: vec![Zoom {
            target: ZoomTarget::Fixed { x: 100.0, y: 200.0 },
            ..doc.zooms[0].clone()
        }],
        ..Default::default()
    };
    assert!(
        !regions_from_doc(&fixed, 1920, 1080)[0].follow_cursor,
        "a Fixed target keeps its stored anchor"
    );
}

#[test]
fn fixed_target_fractions_scale_to_screen_pixels() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let base = Zoom {
        id: "z0".into(),
        start_ms: 0,
        end_ms: 100,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "smooth".into(),
        zoom_in_ms: 350,
        zoom_out_ms: 450,
        layer: 0,
        cam_action: None,
        smart_typing: false,
        easing_out: None,
    };
    let doc_of = |t| EditDoc {
        zooms: vec![Zoom {
            target: t,
            ..base.clone()
        }],
        ..Default::default()
    };
    let frac = regions_from_doc(&doc_of(ZoomTarget::Fixed { x: 0.25, y: 0.75 }), 1920, 1080);
    assert_eq!(frac[0].anchor, FramePoint { x: 480, y: 810 });
    let px = regions_from_doc(
        &doc_of(ZoomTarget::Fixed { x: 1300.0, y: 40.0 }),
        1920,
        1080,
    );
    assert_eq!(px[0].anchor, FramePoint { x: 1300, y: 40 });
}

#[test]
fn per_region_durations_flow_into_regions() {
    use crate::edit::model::{Zoom, ZoomTarget};
    let mut doc = EditDoc::default();
    doc.zooms.push(Zoom {
        id: "z0".into(),
        start_ms: 0,
        end_ms: 1000,
        target: ZoomTarget::Cursor,
        scale: 2.0,
        easing: "smooth".into(),
        zoom_in_ms: 120,
        zoom_out_ms: 640,
        layer: 0,
        cam_action: None,
        smart_typing: false,
        easing_out: None,
    });
    let regs = regions_from_doc(&doc, 800, 600);
    assert_eq!((regs[0].zoom_in_ms, regs[0].zoom_out_ms), (120, 640));
}

#[test]
fn easing_unknown_falls_back_to_config() {
    let cfg = crate::settings::model::Settings::default()
        .zoom
        .to_zoom_config();
    assert_eq!(
        format!("{:?}", easing_from("bogus", cfg.easing)),
        format!("{:?}", cfg.easing)
    );
    assert_eq!(
        format!("{:?}", easing_from("smooth", cfg.easing)),
        format!("{:?}", Easing::Smooth)
    );
    assert_eq!(
        format!("{:?}", easing_from("linear", cfg.easing)),
        format!("{:?}", Easing::Linear)
    );
}

#[test]
fn spring_maps_to_a_real_spring_variant() {
    let cfg = crate::settings::model::Settings::default()
        .zoom
        .to_zoom_config();
    assert!(matches!(
        easing_from("spring", cfg.easing),
        Easing::Spring { .. }
    ));
    assert_eq!(
        easing_from("spring(300,10)", cfg.easing),
        Easing::Spring {
            stiffness: 300.0,
            damping: 10.0,
            mass: 1.0
        }
    );
    assert_eq!(
        easing_from("spring(300,10,2)", cfg.easing),
        Easing::Spring {
            stiffness: 300.0,
            damping: 10.0,
            mass: 2.0
        }
    );
    assert_eq!(easing_from("spring(300)", cfg.easing), cfg.easing);
}

#[test]
fn a_keys_string_rebuilds_its_curve() {
    let cfg = crate::settings::model::Settings::default()
        .zoom
        .to_zoom_config();
    let e = easing_from("keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)", cfg.easing);
    match e {
        Easing::Keys(k) => {
            assert_eq!(k.n, 2);
            assert!((crate::export::keys::eval(&k, 0.5) - 0.5).abs() < 1e-4);
        }
        other => panic!("expected Keys, got {other:?}"),
    }
    assert_eq!(easing_from("keys(0 0 0 0 0 0 b)", cfg.easing), cfg.easing);
}
