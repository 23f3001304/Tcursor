use super::*;
use crate::export::camera::ease;

fn peak(name: &str) -> f32 {
    (0..=2000)
        .map(|i| ease(easing_from(name, Easing::Smooth), i as f32 / 2000.0))
        .fold(f32::MIN, f32::max)
}

#[test]
fn spring_ease_matches_the_ts_mirror_and_overshoots() {
    let want = [
        (0.2_f32, 1.085405_f32),
        (0.3, 1.145100),
        (0.5, 0.975458),
        (0.9, 1.002020),
    ];
    let mut saw_overshoot = false;
    for (p, expected) in want {
        let actual = ease(easing_from("spring", Easing::Smooth), p);
        assert!(
            (actual - expected).abs() < 1e-4,
            "p={p}: {actual} != {expected}"
        );
        saw_overshoot |= actual > 1.0;
    }
    assert!(
        saw_overshoot,
        "at least one sampled p must overshoot past 1.0"
    );
    assert!(
        peak("spring(300,10)") > peak("spring") && peak("spring") > peak("spring(170,60)"),
        "overshoot must track the damping ratio: {} {} {}",
        peak("spring(300,10)"),
        peak("spring"),
        peak("spring(170,60)")
    );
    assert!(
        (peak("spring(170,60)") - 1.0).abs() < 1e-4,
        "an overdamped spring must not overshoot"
    );
}

#[test]
fn spring_layout_transition_overshoots_the_destination_scene() {
    use crate::actions::model::LayoutId;
    use crate::edit::model::LayoutSeg;
    use crate::export::scene::layout::LayoutTrack;
    use crate::export::scene::resolve;
    use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings};

    let app = AppearanceSettings::default();
    let (ow, oh, sw, sh) = (3840u32, 2160u32, 1920u32, 1080u32);
    let scene_of = |id: LayoutId| {
        resolve(
            id,
            &layout_for(app.for_id(id), ow, oh),
            &overlay_for(app.for_id(id), ow, oh, true),
            sw,
            sh,
        )
    };
    let camera = scene_of(LayoutId::Camera);
    let base = LayoutSeg {
        id: "l0".into(),
        start_ms: 0,
        end_ms: 1000,
        layout: "screen".into(),
        transition_ms: 0,
        easing: "smooth".into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    };
    let into = |easing: &str| LayoutSeg {
        id: "l1".into(),
        start_ms: 1000,
        end_ms: 5000,
        layout: "camera".into(),
        transition_ms: 400,
        easing: easing.into(),
        transition_out_ms: 0,
        easing_out: "smooth".into(),
        arrangement: None,
    };
    let spring_track =
        LayoutTrack::from_segs(&[base.clone(), into("spring")], &app, ow, oh, sw, sh);
    let base2 = base.clone();
    let smooth_track = LayoutTrack::from_segs(&[base, into("smooth")], &app, ow, oh, sw, sh);

    let mut spring_overshot = false;
    for t in (1000..1400).step_by(5) {
        if spring_track.scene_at(t).screen.rect.w < camera.screen.rect.w - 1e-3 {
            spring_overshot = true;
        }
        assert!(
            smooth_track.scene_at(t).screen.rect.w >= camera.screen.rect.w - 1e-3,
            "smooth must never overshoot (t={t})"
        );
    }
    assert!(
        spring_overshot,
        "spring must overshoot past the destination width at some t"
    );

    let past = |easing: &str| {
        let tr = LayoutTrack::from_segs(&[base2.clone(), into(easing)], &app, ow, oh, sw, sh);
        (1000..1400)
            .step_by(5)
            .map(|t| camera.screen.rect.w - tr.scene_at(t).screen.rect.w)
            .fold(f32::MIN, f32::max)
    };
    let (loose, tight) = (past("spring(300,10)"), past("spring(170,26)"));
    assert!(
        loose > tight,
        "a lower damping ratio must overshoot further: {loose} vs {tight}"
    );
    assert!(
        tight < 1.0,
        "an all-but-critical spring should barely pass the destination: {tight}"
    );
}
