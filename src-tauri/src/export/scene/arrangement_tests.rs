// Preset-parity tests for pose-based arrangements (T34 L1 step 2), in their own file so
// arrangement.rs stays under the size limit. The load-bearing claim: converting a preset to an
// arrangement and resolving THAT must reproduce the preset's own pixels, or "preset as a starting
// point" would visibly jump the frame the moment the user touched a panel.
use super::*;
use crate::actions::model::LayoutId;
use crate::export::scene::resolve;
use crate::settings::appearance::{layout_for, overlay_for, AppearanceSettings, CamAspect, CamRing,
    CamShape, ModeAppearance};

const PRESETS: [LayoutId; 5] = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter,
    LayoutId::ScreenOnly, LayoutId::CameraOnly];
const OW: u32 = 1920;
const OH: u32 = 1080;
const SW: u32 = 1920;
const SH: u32 = 1080;

/// Every mode sharing one appearance block, so `for_id` returns it whichever preset is under test.
fn uniform(ma: ModeAppearance) -> AppearanceSettings {
    AppearanceSettings { screen: ma, screen_only: ma, camera: ma, camera_only: ma, presenter: ma }
}

/// Resolve `id` the way `LayoutTrack`/`FrameRenderer::resolve_layout` do, plus the `Layout` +
/// `OverlayLayout` that resolution used (an arrangement needs the same two to resolve against).
fn resolved(app: &AppearanceSettings, id: LayoutId) -> (Scene, crate::export::types::Layout,
                                                        crate::export::types::OverlayLayout) {
    let ma = app.for_id(id);
    let (l, ov) = (layout_for(ma, OW, OH), overlay_for(ma, OW, OH, true));
    (resolve(id, &l, &ov, SW, SH), l, ov)
}

/// Largest absolute per-field difference between two scenes, in output pixels (alpha counts as
/// 1.0 == one pixel of error so a visibility mismatch can never hide inside the tolerance).
fn max_dev(a: &Scene, b: &Scene) -> f32 {
    let per = |p: &Panel, q: &Panel| [
        (p.rect.x - q.rect.x).abs(), (p.rect.y - q.rect.y).abs(),
        (p.rect.w - q.rect.w).abs(), (p.rect.h - q.rect.h).abs(),
        (p.radius - q.radius).abs(), (p.alpha - q.alpha).abs(), (p.ring_px - q.ring_px).abs(),
    ].into_iter().fold(0.0f32, f32::max);
    per(&a.screen, &b.screen).max(per(&a.camera, &b.camera))
}

/// The round trip under test: preset -> arrangement -> scene.
fn round_trip(app: &AppearanceSettings, id: LayoutId) -> (Scene, Scene) {
    let (preset, l, ov) = resolved(app, id);
    let arr = arrangement_of_preset(&preset, OW as f32, OH as f32);
    (preset, resolve_arrangement(&arr, preset, &l, &ov, SW, SH))
}

fn assert_parity(app: &AppearanceSettings, tag: &str) -> f32 {
    let mut worst = 0.0f32;
    for id in PRESETS {
        let (preset, posed) = round_trip(app, id);
        let d = max_dev(&preset, &posed);
        assert!(d < 0.5, "{tag} {id:?}: {d}px deviation\n preset {preset:?}\n posed  {posed:?}");
        assert_eq!(preset.camera.ring_color, posed.camera.ring_color, "{tag} {id:?} ring color");
        worst = worst.max(d);
    }
    worst
}

/// Step 2, default appearance: all five presets round-trip within 0.5px at 1920x1080.
#[test]
fn every_preset_round_trips_through_a_pose_within_half_a_pixel() {
    let worst = assert_parity(&AppearanceSettings::default(), "default");
    println!("PARITY default appearance: max deviation {worst}px");
}

/// ...and the parity is not an accident of the defaults: a fully re-styled appearance (thick ring,
/// rounded corners, top-right bubble, shrunken screen, fat padding) round-trips just as tightly,
/// which is what proves radius/ring/shape really do come from the same appearance plumbing.
#[test]
fn parity_survives_a_fully_customised_appearance() {
    let app = uniform(ModeAppearance {
        pad: 0.06, screen_size: 0.8, screen_radius: 0.03, cam_size: 0.28,
        cam_shape: CamShape::Rounded, cam_radius: 0.12,
        cam_corner: crate::settings::appearance::CamCorner::TopRight,
        cam_margin_x: 0.05, cam_margin_y: 0.08, cam_aspect: CamAspect::Square,
        cam_ring: Some(CamRing { width: 0.04, color: [200, 40, 90] }),
    });
    let worst = assert_parity(&app, "custom");
    println!("PARITY custom appearance: max deviation {worst}px");
}

/// A Wide (16:9) webcam keeps parity for the two BUBBLE presets, which are the ones `resolve`
/// builds through `bubble_rect` (the only place it honors `width_px`).
#[test]
fn a_wide_webcam_keeps_parity_for_the_bubble_presets() {
    let app = uniform(ModeAppearance { cam_aspect: CamAspect::Wide, ..ModeAppearance::default() });
    for id in [LayoutId::Screen, LayoutId::ScreenOnly] {
        let (preset, posed) = round_trip(&app, id);
        let d = max_dev(&preset, &posed);
        assert!(d < 0.5, "wide {id:?}: {d}px");
        println!("PARITY wide bubble {id:?}: max deviation {d}px");
    }
}

/// KNOWN, DOCUMENTED divergence: `resolve` draws the BIG camera modes as a `centered_square` of
/// `size_px`, ignoring `width_px` entirely, so under a Wide appearance the preset itself is square
/// while an arrangement honors the configured 16:9. Height and center still agree exactly - only
/// the width does not - so this pins the shape of the gap rather than pretending it isn't there.
#[test]
fn a_wide_webcam_widens_the_big_camera_presets_which_resolve_draws_square() {
    let app = uniform(ModeAppearance { cam_aspect: CamAspect::Wide, ..ModeAppearance::default() });
    for id in [LayoutId::Camera, LayoutId::CameraOnly, LayoutId::Presenter] {
        let (preset, posed) = round_trip(&app, id);
        let (p, q) = (preset.camera.rect, posed.camera.rect);
        assert!((p.h - q.h).abs() < 0.5, "{id:?} height must still match: {} vs {}", p.h, q.h);
        assert!((p.x + p.w / 2.0 - (q.x + q.w / 2.0)).abs() < 0.5, "{id:?} center must still match");
        assert!((q.w / p.w - 16.0 / 9.0).abs() < 0.01,
            "{id:?} the arrangement is the configured 16:9 while the preset is square: {} vs {}", q.w, p.w);
        // The SCREEN panel is untouched by the cam aspect, so it stays pixel-identical.
        assert!((preset.screen.rect.w - posed.screen.rect.w).abs() < 0.5);
    }
}

/// Poses can never stretch content: whatever `size` (height) a pose carries, the resolved width is
/// exactly `height * aspect` - the screen's is the SOURCE aspect, so a 4:3 capture stays 4:3.
#[test]
fn a_pose_derives_width_from_the_panels_own_aspect_and_never_stretches() {
    let app = AppearanceSettings::default();
    let (base, l, ov) = resolved(&app, LayoutId::Screen);
    for (sw, sh) in [(1920u32, 1080u32), (1280, 1024), (2560, 1080)] {
        for size in [0.2f32, 0.5, 1.0, 1.4] {
            let arr = Arrangement { screen: Some(PanelPose { cx: 0.5, cy: 0.5, size }), cam: None };
            let s = resolve_arrangement(&arr, base, &l, &ov, sw, sh).screen.rect;
            assert!((s.h - size * OH as f32).abs() < 0.01, "height is the pose's own");
            assert!((s.w / s.h - sw as f32 / sh as f32).abs() < 0.001,
                "{sw}x{sh} @ {size}: aspect drifted to {}", s.w / s.h);
        }
    }
}

/// A `None` panel is not shown - alpha 0 - but KEEPS the preset's rect, so a cross-dissolve into
/// or out of it slides the panel rather than popping it in from a degenerate rect.
#[test]
fn a_none_panel_is_alpha_zero_at_the_presets_own_rect() {
    let app = AppearanceSettings::default();
    let (base, l, ov) = resolved(&app, LayoutId::Screen);
    let arr = Arrangement { screen: Some(PanelPose { cx: 0.5, cy: 0.5, size: 0.5 }), cam: None };
    let s = resolve_arrangement(&arr, base, &l, &ov, SW, SH);
    assert_eq!(s.camera.alpha, 0.0);
    assert_eq!(s.camera.rect, base.camera.rect, "hidden panel keeps the preset rect for the blend");
    assert_eq!(s.screen.alpha, 1.0);
}

/// A posed panel is SHOWN even when the preset it is based on hid that panel (a `CameraOnly`
/// arrangement that gives the screen a pose must un-hide it), and the conversion the other way maps
/// a preset's hidden panels to `None`.
#[test]
fn hidden_preset_panels_map_to_none_and_a_pose_un_hides_them() {
    let app = AppearanceSettings::default();
    let (screen_only, _, _) = resolved(&app, LayoutId::ScreenOnly);
    let (camera_only, l, ov) = resolved(&app, LayoutId::CameraOnly);
    assert_eq!(arrangement_of_preset(&screen_only, OW as f32, OH as f32).cam, None);
    assert_eq!(arrangement_of_preset(&camera_only, OW as f32, OH as f32).screen, None);

    let arr = Arrangement { screen: Some(PanelPose { cx: 0.2, cy: 0.8, size: 0.3 }),
        cam: Some(PanelPose { cx: 0.5, cy: 0.5, size: 0.9 }) };
    let s = resolve_arrangement(&arr, camera_only, &l, &ov, SW, SH);
    assert_eq!(s.screen.alpha, 1.0, "a posed screen shows even on a CameraOnly base");
    assert!((s.screen.rect.x + s.screen.rect.w / 2.0 - 0.2 * OW as f32).abs() < 0.01);
}

/// The cam's radius and ring scale with the pose exactly as a camera keyframe scales them
/// (`override_camera`), so a circle stays a circle and the ring keeps its proportion.
#[test]
fn a_posed_camera_scales_its_radius_and_ring_like_a_keyframe_does() {
    let app = uniform(ModeAppearance { cam_ring: Some(CamRing { width: 0.05, color: [10, 20, 30] }),
        ..ModeAppearance::default() });
    let (base, l, ov) = resolved(&app, LayoutId::Screen);
    let half = base.camera.rect.h / OH as f32 / 2.0;
    let arr = Arrangement { screen: None, cam: Some(PanelPose { cx: 0.5, cy: 0.5, size: half }) };
    let cam = resolve_arrangement(&arr, base, &l, &ov, SW, SH).camera;
    assert!((cam.radius - base.camera.radius / 2.0).abs() < 0.01, "circle must stay a circle");
    assert!((cam.ring_px - base.camera.ring_px / 2.0).abs() < 0.01, "ring keeps its proportion");
    assert!(cam.ring_px > 0.0 && cam.ring_color == [10, 20, 30], "ring comes from the appearance");
}
