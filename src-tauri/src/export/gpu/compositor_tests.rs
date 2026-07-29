// Tests for export::gpu::compositor, split out so compositor.rs stays under the size limit.
use super::*;
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, Layout, RectF};

fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    let mut v = vec![0u8; (w * h * 4) as usize];
    for c in v.chunks_mut(4) { c.copy_from_slice(&px); } v
}
/// A solid-color screen frame in the nv12 layout `composite_into` now expects (Y + half-res UV).
fn nv12_screen(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    crate::export::color::bgra_to_nv12(&solid(w, h, px), w, h)
}
fn panel(x: f32, y: f32, w: f32, h: f32, a: f32) -> Panel {
    Panel { rect: RectF { x, y, w, h }, radius: 0.0, alpha: a, ring_px: 0.0, ring_color: [0, 0, 0] }
}

#[test]
fn screen_panel_composites_onto_background() {
    // 4x4 red screen into a 4x4 panel at (2,2) of an 8x8 blue output, no zoom, no camera.
    let screen = nv12_screen(4, 4, [0, 0, 255, 255]); // BGRA red -> nv12
    let bg = solid(8, 8, [255, 0, 0, 255]);      // BGRA blue
    let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
    let scene = Scene { screen: panel(2.0, 2.0, 4.0, 4.0, 1.0), camera: panel(0.0, 0.0, 0.0, 0.0, 0.0) };
    let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
    let mut out = Vec::new();
    CpuCompositor.composite_into(&screen, 4, 4, None, cam, &bg, &layout, &scene, &mut out);
    assert_eq!(out.len(), 8 * 8 * 4);
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);              // corner = bg blue (unchanged)
    let i = ((3 * 8 + 3) * 4) as usize;
    let p = &out[i..i + 4]; // panel interior ~= screen red (nv12 round-trip is exact to a few LSBs)
    assert!(p[0] <= 3 && p[1] <= 3 && p[2] >= 250, "panel interior must be ~screen red, got {p:?}");
}

#[test]
fn ring_paints_a_band_just_inside_the_camera_edge_and_leaves_center_alone() {
    // 20x20 square (no rounding) camera panel with a 3px ring; source is solid green.
    let webcam = solid(20, 20, [0, 255, 0, 255]); // BGRA green
    let screen = nv12_screen(4, 4, [0, 0, 0, 255]);
    let bg = solid(24, 24, [50, 50, 50, 255]);
    let layout = Layout { out_w: 24, out_h: 24, pad_px: 1, screen_scale: 1.0, screen_radius_px: 0.0 };
    let mut cam_panel = panel(2.0, 2.0, 20.0, 20.0, 1.0);
    cam_panel.ring_px = 3.0;
    cam_panel.ring_color = [255, 0, 0]; // RGB red -> BGRA [0,0,255,255]
    let scene = Scene { screen: panel(0.0, 0.0, 0.0, 0.0, 0.0), camera: cam_panel };
    let cam = Camera { cx: 12.0, cy: 12.0, scale: 1.0 };
    let mut out = Vec::new();
    CpuCompositor.composite_into(&screen, 4, 4, Some((&webcam, 20, 20)), cam, &bg, &layout, &scene, &mut out);
    // Panel spans output (2,2)..(22,22), local x=0 at output x=2. At the outer edge the SDF
    // band is antialiased (never exactly 1.0 at a pixel center, same as the shader/blit
    // feathering elsewhere), so assert it is STRONGLY ring-tinted (mostly red, little green)
    // rather than exactly pure red: band = (ring_px + d)/ring_px at d=-0.5, ring_px=3 -> 0.8333.
    let edge = ((12 * 24 + 2) * 4) as usize;
    let px = &out[edge..edge + 4];
    assert_eq!(px[0], 0, "no blue component");
    assert!(px[1] < 60, "green should be mostly suppressed by the ring band, got {}", px[1]); // 255*(1-0.8333) ~= 42
    assert!(px[2] > 200, "red (ring color) should dominate, got {}", px[2]);
    // Center pixel (12,12) is far from the edge -> untouched green webcam.
    let center = ((12 * 24 + 12) * 4) as usize;
    assert_eq!(&out[center..center + 4], &[0, 255, 0, 255], "center should stay webcam green");
}

#[test]
fn zero_ring_px_leaves_panel_byte_identical_to_no_ring_field() {
    let webcam = solid(10, 10, [10, 20, 30, 255]);
    let screen = nv12_screen(4, 4, [0, 0, 0, 255]);
    let bg = solid(14, 14, [40, 40, 40, 255]);
    let layout = Layout { out_w: 14, out_h: 14, pad_px: 1, screen_scale: 1.0, screen_radius_px: 0.0 };
    let scene = Scene { screen: panel(0.0, 0.0, 0.0, 0.0, 0.0), camera: panel(2.0, 2.0, 10.0, 10.0, 1.0) };
    let cam = Camera { cx: 7.0, cy: 7.0, scale: 1.0 };
    let mut baseline = Vec::new();
    CpuCompositor.composite_into(&screen, 4, 4, Some((&webcam, 10, 10)), cam, &bg, &layout, &scene, &mut baseline);
    // Same scene, but camera.ring_px explicitly 0.0 with a non-black color set - must be a no-op.
    let mut cam_panel = panel(2.0, 2.0, 10.0, 10.0, 1.0);
    cam_panel.ring_color = [200, 100, 50];
    let scene2 = Scene { screen: panel(0.0, 0.0, 0.0, 0.0, 0.0), camera: cam_panel };
    let mut out = Vec::new();
    CpuCompositor.composite_into(&screen, 4, 4, Some((&webcam, 10, 10)), cam, &bg, &layout, &scene2, &mut out);
    assert_eq!(out, baseline, "ring_px == 0.0 must never touch pixels, regardless of ring_color");
}

#[test]
fn disabled_and_degenerate_panels_do_not_panic() {
    let screen = nv12_screen(8, 8, [0, 0, 255, 255]);
    let webcam = solid(4, 4, [0, 255, 0, 255]);
    let bg = solid(8, 8, [255, 0, 0, 255]);
    let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
    // camera panel larger than output + screen disabled: must not OOB or panic.
    let scene = Scene { screen: panel(0.0, 0.0, 8.0, 8.0, 0.0), camera: panel(2.0, 2.0, 20.0, 20.0, 1.0) };
    let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
    let mut out = Vec::new();
    CpuCompositor.composite_into(&screen, 8, 8, Some((&webcam, 4, 4)), cam, &bg, &layout, &scene, &mut out);
    assert_eq!(out.len(), 8 * 8 * 4);
}

#[test]
fn blit_opaque_inner_skip_is_byte_identical_to_full_sdf() {
    // A 40x40 rounded (r=6) opaque panel blitted onto a 48x48 background at (4,4).
    // Fast-path (opaque inner rect forced to a=1.0) must equal full-SDF (None) exactly,
    // INCLUDING the antialiased corners outside the inner rect.
    let (pw, ph, r) = (40u32, 40u32, 6.0f32);
    // Non-uniform src so any wrong blend/copy would show.
    let mut src = vec![0u8; (pw * ph * 4) as usize];
    for (i, px) in src.chunks_mut(4).enumerate() {
        px.copy_from_slice(&[(i % 251) as u8, (i * 3 % 251) as u8, (i * 7 % 251) as u8, 255]);
    }
    let (dw, dh, ox, oy) = (48u32, 48u32, 4u32, 4u32);
    let (hw, hh) = (pw as f32 / 2.0, ph as f32 / 2.0);
    let cov = move |tx: u32, ty: u32| {
        let qx = ((tx as f32 + 0.5) - hw).abs() - (hw - r);
        let qy = ((ty as f32 + 0.5) - hh).abs() - (hh - r);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        let d = qx.max(qy).min(0.0) + outside - r;
        (0.5 - d).clamp(0.0, 1.0) * 1.0
    };
    let ins = (r.ceil() as u32) + 2;
    let inner = (pw > 2 * ins && ph > 2 * ins).then(|| (ins, ins, pw - ins, ph - ins));
    assert!(inner.is_some(), "inner rect must be non-empty for this test to mean anything");
    let base = vec![30u8; (dw * dh * 4) as usize];
    let mut fast = base.clone();
    blit(&mut fast, dw, dh, &src, pw, ph, ox, oy, inner, cov);
    let mut full = base.clone();
    blit(&mut full, dw, dh, &src, pw, ph, ox, oy, None, cov);
    assert_eq!(fast, full, "opaque-inner skip diverged from full SDF");
}
