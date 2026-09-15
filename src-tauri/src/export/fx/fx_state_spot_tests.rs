use super::*;

#[test]
fn spotlight_toggle_makes_a_spot_at_cursor() {
    let s = fx_state_at(
        &fx(ClickFxStyle::None, true),
        &[],
        &[],
        &[],
        &full_scene(100, 100),
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s.spot.unwrap();
    assert!((spot.alpha - 1.0).abs() < 1e-6);
    assert!((spot.cx - 50.0).abs() < 1.0 && (spot.cy - 50.0).abs() < 1.0);
    assert!(s.hits.is_empty());
    assert!((spot.radius_frac - 0.13).abs() < 1e-6);
}

#[test]
fn spotlight_radius_scales_with_screen_panel_height() {
    let half = Scene {
        screen: Panel {
            rect: RectF {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            },
            radius: 0.0,
            alpha: 1.0,
            ring_px: 0.0,
            ring_color: [0, 0, 0],
        },
        camera: Panel {
            rect: RectF {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            radius: 0.0,
            alpha: 0.0,
            ring_px: 0.0,
            ring_color: [0, 0, 0],
        },
        src: crate::export::coordmap::full_src(100, 100),
    };
    let s = fx_state_at(
        &fx(ClickFxStyle::None, true),
        &[],
        &[],
        &[],
        &half,
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s.spot.unwrap();
    assert!(
        (spot.radius_frac - 0.13 * 0.5).abs() < 1e-6,
        "radius_frac should scale by screen height fraction: {}",
        spot.radius_frac
    );
    assert!(
        (spot.feather_frac - 0.10 * 0.5).abs() < 1e-6,
        "feather_frac should scale too: {}",
        spot.feather_frac
    );
}

#[test]
fn spotlight_hole_disabled_without_a_real_webcam() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false;
    let mut scene = full_scene(100, 100);
    scene.camera.alpha = 1.0;
    let s = fx_state_at(
        &settings,
        &[],
        &[],
        &[],
        &scene,
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        false,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s.spot.unwrap();
    assert!(
        spot.dim_camera,
        "no webcam.mp4 -> the hole must not apply, regardless of the user's setting"
    );
}

#[test]
fn spotlight_hole_disabled_when_the_camera_panel_is_invisible() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false;
    let scene = full_scene(100, 100);
    let s = fx_state_at(
        &settings,
        &[],
        &[],
        &[],
        &scene,
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s.spot.unwrap();
    assert!(
        spot.dim_camera,
        "camera.alpha <= 0.05 -> the hole must not apply even with a real webcam"
    );
}

#[test]
fn spotlight_hole_enabled_with_a_real_visible_webcam() {
    let mut settings = fx(ClickFxStyle::None, true);
    settings.spotlight_dim_camera = false;
    let mut scene = full_scene(100, 100);
    scene.camera.alpha = 1.0;
    let s = fx_state_at(
        &settings,
        &[],
        &[],
        &[],
        &scene,
        cam(),
        FramePoint { x: 50, y: 50 },
        &scr(),
        true,
        100,
        100,
        0,
        0,
        &mut SpotlightSim::new(),
    )
    .unwrap();
    let spot = s.spot.unwrap();
    assert!(
        !spot.dim_camera,
        "a real, visible webcam + dim_camera:false -> the hole DOES apply"
    );
}
