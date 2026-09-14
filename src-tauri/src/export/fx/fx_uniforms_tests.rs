// Split out of `fx_uniforms.rs` when the glass-lens slots pushed that file over the size limit.
use super::*;
use crate::export::fx::fx_state::{FxHit, FxState, Spot};
use crate::settings::model::{ClickFxStyle, SpotlightMode};

#[test]
fn style_id_covers_all_variants() {
    use crate::settings::model::ClickFxStyle::*;
    assert_eq!(style_id(None), 0.0);
    assert_eq!(style_id(Ripple), 1.0);
    assert_eq!(style_id(Pulse), 2.0);
    assert_eq!(style_id(Glow), 3.0);
    assert_eq!(style_id(Shockwave), 4.0);
    assert_eq!(style_id(Particles), 5.0);
    assert_eq!(style_id(Neon), 6.0);
}
#[test]
fn maps_style_hits_spot_and_color() {
    let st = FxState { style: ClickFxStyle::Ripple, color: [255, 0, 0], intensity: 0.8,
        hits: vec![FxHit { x: 10.0, y: 20.0, progress: 0.4 }],
        spot: Some(Spot { cx: 5.0, cy: 6.0, dim: 0.5, radius_frac: 0.1, feather_frac: 0.1, alpha: 1.0,
            mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0,
            cam_rect: [0.0; 4], cam_radius: 0.0, dim_camera: true }), video: None, lens: None };
    let u = build_fx_u(&st, 1000, 2000);
    assert_eq!(u.a, [1000.0, 2000.0, 1.0, 1.0]);     // ow, oh, style=ripple, 1 hit
    assert_eq!(u.b[3], 1.0);                          // spot active
    assert!((u.b[2] - 0.5).abs() < 1e-6);             // dim*alpha
    assert!((u.color[0] - 1.0).abs() < 1e-6 && u.color[1] < 1e-6); // red normalized
    assert_eq!(u.hits[0], [10.0, 20.0, 0.4, 0.0]);
    // color2 rides alongside color: pure red rotated 30 deg is orange (1, 0.5, 0).
    assert!((u.color2[0] - 1.0).abs() < 1e-6 && (u.color2[1] - 0.5).abs() < 1e-6 && u.color2[2] < 1e-6);
}
#[test]
fn hue_shift_rotates_and_wraps() {
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
    assert!(near(hue_shift([255, 0, 0], 30.0), [1.0, 0.5, 0.0]), "red -> orange");
    assert!(near(hue_shift([255, 0, 0], 120.0), [0.0, 1.0, 0.0]), "red -> green");
    assert!(near(hue_shift([0, 0, 255], 120.0), [1.0, 0.0, 0.0]), "blue wraps past 360 -> red");
    assert!(near(hue_shift([255, 0, 0], 0.0), [1.0, 0.0, 0.0]), "no rotation is a no-op");
    // Greys have no hue to rotate, and must not pick one up (c == 0 -> s == 0).
    assert!(near(hue_shift([128, 128, 128], 30.0), [128.0 / 255.0; 3]));
    assert!(near(hue_shift([0, 0, 0], 90.0), [0.0; 3]));
}
#[test]
fn neon_hue_shift_is_pinned_at_30_degrees() {
    assert_eq!(NEON_HUE_SHIFT, 30.0); // fx_clicks.wgsl's comment and clickdraw.rs both cite it
}
#[test]
fn no_spot_sets_inactive() {
    let st = FxState { style: ClickFxStyle::None, color: [0,0,0], intensity: 1.0, hits: vec![], spot: None, video: None, lens: None };
    assert_eq!(build_fx_u(&st, 8, 8).b[3], 0.0);
}
#[test]
fn spot_mode_id_and_tint_pack() {
    use crate::settings::model::SpotlightMode::*;
    assert_eq!(spot_mode_id(Classic), 0.0);
    assert_eq!(spot_mode_id(Nebula), 4.0);
    assert_eq!(spot_mode_id(Vignette), 5.0);
    let st = crate::export::fx::fx_state::FxState { style: crate::settings::model::ClickFxStyle::None,
        color: [0,0,0], intensity: 1.0, hits: vec![],
        spot: Some(crate::export::fx::fx_state::Spot { cx: 1.0, cy: 2.0, dim: 0.5, radius_frac: 0.1,
            feather_frac: 0.1, alpha: 1.0, mode: Nebula, tint: [255, 0, 128], t: 3.0,
            cam_rect: [0.0; 4], cam_radius: 0.0, dim_camera: true }), video: None, lens: None };
    let u = build_fx_u(&st, 100, 100);
    assert_eq!(u.d[0], 4.0);                 // mode = nebula
    assert!((u.d[1] - 3.0).abs() < 1e-6);    // time
    assert!((u.tint[0] - 1.0).abs() < 1e-6 && u.tint[2] > 0.49); // tint r=1, b~0.5
}
#[test]
fn dim_camera_false_sets_keep_flag_and_cam_rect() {
    let st = FxState { style: ClickFxStyle::None, color: [0, 0, 0], intensity: 1.0, hits: vec![],
        spot: Some(Spot { cx: 1.0, cy: 2.0, dim: 0.5, radius_frac: 0.1, feather_frac: 0.1, alpha: 1.0,
            mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0,
            cam_rect: [10.0, 20.0, 110.0, 220.0], cam_radius: 8.0, dim_camera: false }), video: None, lens: None };
    let u = build_fx_u(&st, 200, 200);
    assert_eq!(u.d[2], 1.0, "dim_camera:false -> keep-camera-lit flag set");
    assert_eq!(u.d[3], 8.0, "cam_radius packed into d[3]");
    assert_eq!(u.cam, [10.0, 20.0, 110.0, 220.0], "cam rect packed verbatim");
}
#[test]
fn dim_camera_true_clears_keep_flag() {
    let st = FxState { style: ClickFxStyle::None, color: [0, 0, 0], intensity: 1.0, hits: vec![],
        spot: Some(Spot { cx: 1.0, cy: 2.0, dim: 0.5, radius_frac: 0.1, feather_frac: 0.1, alpha: 1.0,
            mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0,
            cam_rect: [0.0; 4], cam_radius: 0.0, dim_camera: true }), video: None, lens: None };
    let u = build_fx_u(&st, 100, 100);
    assert_eq!(u.d[2], 0.0, "dim_camera:true -> keep-camera-lit flag clear (today's behavior)");
}
