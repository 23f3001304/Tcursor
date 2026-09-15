use super::*;
use crate::export::fx::fxdraw::CpuFx;

fn halo_neon_overlay(fx: &dyn FxRenderer) -> String {
    render_fx_overlay(
        fx,
        64,
        64,
        true,
        "neon".into(),
        [0, 128, 255],
        1.0,
        vec![[32.0, 32.0, 0.5]],
        Some(32.0),
        Some(32.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("halo".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn preview_renders_with_the_exports_renderer_selection() {
    let via_command = with_fx(64, 64, halo_neon_overlay);
    let via_select = halo_neon_overlay(select_fx(64, 64).as_ref());
    assert_eq!(
        via_command, via_select,
        "the overlay must use select_fx's renderer, not a fixed one"
    );
}

fn render_empty(w: u32, h: u32) -> usize {
    with_fx(w, h, |fx| {
        let mut buf = vec![0u8; (w * h * 4) as usize];
        fx.apply(
            &mut buf,
            w,
            h,
            &FxState {
                style: ClickFxStyle::None,
                color: [0, 0, 0],
                intensity: 1.0,
                hits: vec![],
                spot: None,
                video: None,
                lens: None,
            },
        );
        buf.len()
    })
}

#[test]
fn a_panic_inside_the_render_does_not_disable_later_overlays() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let boom = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_fx(24, 24, |_| -> usize {
            panic!("simulated readback failure")
        })
    }));
    std::panic::set_hook(prev);
    assert!(boom.is_err(), "the panic must propagate, not be swallowed");
    assert_eq!(
        render_empty(24, 24),
        24 * 24 * 4,
        "the overlay still works after a render panic"
    );
    assert_eq!(render_empty(32, 24), 32 * 24 * 4, "and at a different size");
}

#[test]
fn with_fx_rebuilds_when_the_overlay_size_changes() {
    assert_eq!(render_empty(32, 32), 32 * 32 * 4);
    assert_eq!(
        render_empty(48, 16),
        48 * 16 * 4,
        "a different size must not reuse the old renderer"
    );
}

#[test]
fn dim_area_becomes_black_with_matching_alpha() {
    let o = reconstruct(&[0, 0, 0, 255], &[102, 102, 102, 255]);
    assert_eq!((o[0], o[1], o[2]), (0, 0, 0), "dim overlay colour is black");
    assert!(
        (o[3] as i32 - 153).abs() <= 1,
        "alpha ~0.6 -> 153, got {}",
        o[3]
    );
}

#[test]
fn untouched_pixel_is_fully_transparent() {
    let o = reconstruct(&[0, 0, 0, 255], &[255, 255, 255, 255]);
    assert_eq!(
        o[3], 0,
        "a pixel the effect never touched must be transparent"
    );
}

#[test]
fn additive_click_recovers_bright_colour() {
    let o = reconstruct(&[128, 128, 128, 255], &[255, 255, 255, 255]);
    assert!(o[3] > 120 && o[3] < 135, "alpha ~0.5, got {}", o[3]);
    assert!(
        o[0] > 240 && o[1] > 240 && o[2] > 240,
        "colour recovered near full"
    );
}

#[test]
fn spotlight_command_returns_a_png_data_url() {
    let url = render_fx_overlay(
        &CpuFx,
        80,
        80,
        true,
        "none".into(),
        [255, 255, 255],
        1.0,
        vec![],
        Some(40.0),
        Some(40.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("classic".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(
        url.starts_with("data:image/png;base64,"),
        "returns a PNG data URL"
    );
    assert!(url.len() > 200, "non-trivial overlay encoded");
}

#[test]
fn dim_camera_false_is_threaded_into_the_spot() {
    let url_dimmed = render_fx_overlay(
        &CpuFx,
        80,
        80,
        true,
        "none".into(),
        [255, 255, 255],
        1.0,
        vec![],
        Some(40.0),
        Some(40.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("classic".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        Some([0.0, 0.0, 80.0, 80.0]),
        Some(0.0),
        Some(true),
    )
    .unwrap();
    let url_kept = render_fx_overlay(
        &CpuFx,
        80,
        80,
        true,
        "none".into(),
        [255, 255, 255],
        1.0,
        vec![],
        Some(40.0),
        Some(40.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("classic".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        Some([0.0, 0.0, 80.0, 80.0]),
        Some(0.0),
        Some(false),
    )
    .unwrap();
    assert_ne!(
        url_dimmed, url_kept,
        "dim_camera flag must change the rendered overlay"
    );
}

#[test]
fn gate_cam_hole_blocks_a_requested_hole_without_a_webcam() {
    let (rect, radius, dim) =
        gate_cam_hole(false, Some([0.0, 0.0, 80.0, 80.0]), Some(10.0), Some(false));
    assert_eq!(
        rect, None,
        "no webcam -> no hole rect, regardless of what the frontend sent"
    );
    assert_eq!(radius, None);
    assert_eq!(dim, Some(true), "forced back to dim-everything");
}

#[test]
fn gate_cam_hole_passes_through_a_requested_hole_with_a_webcam() {
    let (rect, radius, dim) =
        gate_cam_hole(true, Some([0.0, 0.0, 80.0, 80.0]), Some(10.0), Some(false));
    assert_eq!(rect, Some([0.0, 0.0, 80.0, 80.0]));
    assert_eq!(radius, Some(10.0));
    assert_eq!(dim, Some(false));
}

#[test]
fn gate_cam_hole_is_a_noop_when_no_hole_was_requested() {
    let (rect, radius, dim) = gate_cam_hole(false, None, None, None);
    assert_eq!((rect, radius, dim), (None, None, None));
}

#[test]
fn no_hole_without_webcam_even_if_dim_camera_is_false() {
    let url_no_webcam = render_fx_overlay(
        &CpuFx,
        80,
        80,
        false,
        "none".into(),
        [255, 255, 255],
        1.0,
        vec![],
        Some(40.0),
        Some(40.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("classic".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        Some([0.0, 0.0, 80.0, 80.0]),
        Some(0.0),
        Some(false),
    )
    .unwrap();
    let url_plain_dim = render_fx_overlay(
        &CpuFx,
        80,
        80,
        true,
        "none".into(),
        [255, 255, 255],
        1.0,
        vec![],
        Some(40.0),
        Some(40.0),
        Some(0.6),
        Some(0.13),
        Some(0.10),
        Some(1.0),
        Some("classic".into()),
        Some([130, 90, 255]),
        Some(0.0),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        url_no_webcam, url_plain_dim,
        "no webcam -> the hole must never render"
    );
}

#[path = "preview_fx_alpha_tests.rs"]
mod alpha_tests;
