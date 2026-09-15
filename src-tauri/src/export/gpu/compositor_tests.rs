use super::*;
use crate::export::coordmap::full_src;
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, Layout, RectF};

fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    let mut v = vec![0u8; (w * h * 4) as usize];
    for c in v.chunks_mut(4) {
        c.copy_from_slice(&px);
    }
    v
}

fn nv12_screen(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    crate::export::color::bgra_to_nv12(&solid(w, h, px), w, h)
}
fn panel(x: f32, y: f32, w: f32, h: f32, a: f32) -> Panel {
    Panel {
        rect: RectF { x, y, w, h },
        radius: 0.0,
        alpha: a,
        ring_px: 0.0,
        ring_color: [0, 0, 0],
    }
}

#[test]
fn screen_panel_composites_onto_background() {
    let screen = nv12_screen(4, 4, [0, 0, 255, 255]);
    let bg = solid(8, 8, [255, 0, 0, 255]);
    let layout = Layout {
        out_w: 8,
        out_h: 8,
        pad_px: 1,
        screen_scale: 1.0,
        screen_radius_px: 8.0 * 0.016,
    };
    let scene = Scene {
        screen: panel(2.0, 2.0, 4.0, 4.0, 1.0),
        camera: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        src: full_src(4, 4),
    };
    let cam = Camera {
        cx: 4.0,
        cy: 4.0,
        scale: 1.0,
    };
    let mut out = Vec::new();
    CpuCompositor.composite_into(&screen, 4, 4, None, cam, &bg, &layout, &scene, &mut out);
    assert_eq!(out.len(), 8 * 8 * 4);
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);
    let i = ((3 * 8 + 3) * 4) as usize;
    let p = &out[i..i + 4];
    assert!(
        p[0] <= 3 && p[1] <= 3 && p[2] >= 250,
        "panel interior must be ~screen red, got {p:?}"
    );
}

#[test]
fn cover_rect_crops_the_mismatched_axis_for_every_panel_aspect() {
    let near = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
        assert!(
            [a.0 - b.0, a.1 - b.1, a.2 - b.2, a.3 - b.3]
                .iter()
                .all(|v| v.abs() < 1e-6),
            "got {a:?}, want {b:?}"
        )
    };
    near(cover_rect(1280, 720, 100, 100), (280.0, 0.0, 720.0, 720.0));
    near(cover_rect(1280, 720, 448, 252), (0.0, 0.0, 1280.0, 720.0));
    near(cover_rect(720, 720, 16, 9), (0.0, 157.5, 720.0, 405.0));
    near(
        cover_rect(1080, 1920, 400, 400),
        (0.0, 420.0, 1080.0, 1080.0),
    );
    near(cover_rect(0, 0, 0, 0), (0.0, 0.0, 0.0, 0.0));
}

#[test]
fn a_wide_webcam_in_a_square_panel_shows_the_centre_not_a_squash() {
    let mut webcam = vec![0u8; 12 * 4 * 4];
    for (i, px) in webcam.chunks_mut(4).enumerate() {
        px.copy_from_slice(&match (i % 12) / 4 {
            0 => [255, 0, 0, 255],
            1 => [0, 255, 0, 255],
            _ => [0, 0, 255, 255],
        });
    }
    let screen = nv12_screen(4, 4, [0, 0, 0, 255]);
    let bg = solid(8, 8, [40, 40, 40, 255]);
    let layout = Layout {
        out_w: 8,
        out_h: 8,
        pad_px: 1,
        screen_scale: 1.0,
        screen_radius_px: 0.0,
    };
    let scene = Scene {
        screen: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        camera: panel(2.0, 2.0, 4.0, 4.0, 1.0),
        src: full_src(4, 4),
    };
    let cam = Camera {
        cx: 4.0,
        cy: 4.0,
        scale: 1.0,
    };
    let mut out = Vec::new();
    CpuCompositor.composite_into(
        &screen,
        4,
        4,
        Some((&webcam, 12, 4)),
        cam,
        &bg,
        &layout,
        &scene,
        &mut out,
    );
    let i = ((4 * 8 + 4) * 4) as usize;
    let p = &out[i..i + 4];
    assert!(
        p[1] > 200 && p[0] < 60 && p[2] < 60,
        "square panel must show the centre (green), got {p:?}"
    );
}

#[test]
fn ring_paints_a_band_just_inside_the_camera_edge_and_leaves_center_alone() {
    let webcam = solid(20, 20, [0, 255, 0, 255]);
    let screen = nv12_screen(4, 4, [0, 0, 0, 255]);
    let bg = solid(24, 24, [50, 50, 50, 255]);
    let layout = Layout {
        out_w: 24,
        out_h: 24,
        pad_px: 1,
        screen_scale: 1.0,
        screen_radius_px: 0.0,
    };
    let mut cam_panel = panel(2.0, 2.0, 20.0, 20.0, 1.0);
    cam_panel.ring_px = 3.0;
    cam_panel.ring_color = [255, 0, 0];
    let scene = Scene {
        screen: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        camera: cam_panel,
        src: full_src(4, 4),
    };
    let cam = Camera {
        cx: 12.0,
        cy: 12.0,
        scale: 1.0,
    };
    let mut out = Vec::new();
    CpuCompositor.composite_into(
        &screen,
        4,
        4,
        Some((&webcam, 20, 20)),
        cam,
        &bg,
        &layout,
        &scene,
        &mut out,
    );
    let edge = ((12 * 24 + 2) * 4) as usize;
    let px = &out[edge..edge + 4];
    assert_eq!(px[0], 0, "no blue component");
    assert!(
        px[1] < 60,
        "green should be mostly suppressed by the ring band, got {}",
        px[1]
    );
    assert!(
        px[2] > 200,
        "red (ring color) should dominate, got {}",
        px[2]
    );
    let center = ((12 * 24 + 12) * 4) as usize;
    assert_eq!(
        &out[center..center + 4],
        &[0, 255, 0, 255],
        "center should stay webcam green"
    );
}

#[test]
fn zero_ring_px_leaves_panel_byte_identical_to_no_ring_field() {
    let webcam = solid(10, 10, [10, 20, 30, 255]);
    let screen = nv12_screen(4, 4, [0, 0, 0, 255]);
    let bg = solid(14, 14, [40, 40, 40, 255]);
    let layout = Layout {
        out_w: 14,
        out_h: 14,
        pad_px: 1,
        screen_scale: 1.0,
        screen_radius_px: 0.0,
    };
    let scene = Scene {
        screen: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        camera: panel(2.0, 2.0, 10.0, 10.0, 1.0),
        src: full_src(4, 4),
    };
    let cam = Camera {
        cx: 7.0,
        cy: 7.0,
        scale: 1.0,
    };
    let mut baseline = Vec::new();
    CpuCompositor.composite_into(
        &screen,
        4,
        4,
        Some((&webcam, 10, 10)),
        cam,
        &bg,
        &layout,
        &scene,
        &mut baseline,
    );
    let mut cam_panel = panel(2.0, 2.0, 10.0, 10.0, 1.0);
    cam_panel.ring_color = [200, 100, 50];
    let scene2 = Scene {
        screen: panel(0.0, 0.0, 0.0, 0.0, 0.0),
        camera: cam_panel,
        src: full_src(4, 4),
    };
    let mut out = Vec::new();
    CpuCompositor.composite_into(
        &screen,
        4,
        4,
        Some((&webcam, 10, 10)),
        cam,
        &bg,
        &layout,
        &scene2,
        &mut out,
    );
    assert_eq!(
        out, baseline,
        "ring_px == 0.0 must never touch pixels, regardless of ring_color"
    );
}

#[test]
fn disabled_and_degenerate_panels_do_not_panic() {
    let screen = nv12_screen(8, 8, [0, 0, 255, 255]);
    let webcam = solid(4, 4, [0, 255, 0, 255]);
    let bg = solid(8, 8, [255, 0, 0, 255]);
    let layout = Layout {
        out_w: 8,
        out_h: 8,
        pad_px: 1,
        screen_scale: 1.0,
        screen_radius_px: 8.0 * 0.016,
    };
    let scene = Scene {
        screen: panel(0.0, 0.0, 8.0, 8.0, 0.0),
        camera: panel(2.0, 2.0, 20.0, 20.0, 1.0),
        src: full_src(8, 8),
    };
    let cam = Camera {
        cx: 4.0,
        cy: 4.0,
        scale: 1.0,
    };
    let mut out = Vec::new();
    CpuCompositor.composite_into(
        &screen,
        8,
        8,
        Some((&webcam, 4, 4)),
        cam,
        &bg,
        &layout,
        &scene,
        &mut out,
    );
    assert_eq!(out.len(), 8 * 8 * 4);
}

#[path = "compositor_blit_tests.rs"]
mod blit_tests;
