// Tests for `GpuCompositor` (`gpu_compositor.rs`), split out so that file stays under the
// line budget. Every test no-ops when the machine has no usable wgpu adapter.
use super::*;
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, Layout, RectF};

fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    let mut v = vec![0u8; (w * h * 4) as usize];
    for c in v.chunks_mut(4) { c.copy_from_slice(&px); } v
}

#[test]
fn screen_panel_composites_onto_background() {
    let c = match GpuCompositor::new(8, 8) { Some(c) => c, None => return };
    let screen = crate::export::color::bgra_to_nv12(&solid(4, 4, [0, 0, 255, 255]), 4, 4);
    let bg = solid(8, 8, [255, 0, 0, 255]);
    let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
    let scene = Scene {
        screen: Panel { rect: RectF { x: 2.0, y: 2.0, w: 4.0, h: 4.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0, ring_px: 0.0, ring_color: [0, 0, 0] },
    };
    let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
    let mut out = Vec::new();
    c.composite_into(&screen, 4, 4, None, cam, &bg, &layout, &scene, &mut out);
    assert_eq!(out.len(), 8 * 8 * 4);
    assert_eq!(&out[0..4], &[255, 0, 0, 255], "corner must be bg blue");
    let i = ((3 * 8 + 3) * 4) as usize;
    let p = &out[i..i + 4]; // screen red through the nv12 shader convert (exact to a few LSBs)
    assert!(p[0] <= 3 && p[1] <= 3 && p[2] >= 250, "panel interior must be ~screen red, got {p:?}");
}

/// R3: ONE webcam decode box, cropped per panel at composite time - and both compositors must
/// crop it the SAME way. A 3:1 webcam in a square panel shows the middle third at true
/// proportions on the GPU exactly as on the CPU (`cover_rect` vs the shader's `cover_uv`).
#[test]
fn cpu_gpu_parity_cover_crops_a_wide_webcam_into_a_square_panel() {
    use crate::export::gpu::compositor::{Compositor, CpuCompositor};
    let g = match GpuCompositor::new(32, 24) { Some(c) => c, None => return };
    let mut webcam = vec![0u8; 24 * 8 * 4];
    for y in 0..8usize {
        for x in 0..24usize {
            let px: [u8; 4] = match x / 8 { 0 => [255, 0, 0, 255], 1 => [0, 255, 0, 255], _ => [0, 0, 255, 255] };
            webcam[(y * 24 + x) * 4..(y * 24 + x) * 4 + 4].copy_from_slice(&px);
        }
    }
    let screen = crate::export::color::bgra_to_nv12(&solid(16, 12, [10, 10, 10, 255]), 16, 12);
    let bg = solid(32, 24, [40, 40, 40, 255]);
    let layout = Layout { out_w: 32, out_h: 24, pad_px: 2, screen_scale: 1.0, screen_radius_px: 0.0 };
    let scene = Scene {
        screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        camera: Panel { rect: RectF { x: 12.0, y: 8.0, w: 8.0, h: 8.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
    };
    let cam = Camera { cx: 16.0, cy: 12.0, scale: 1.0 };
    let (mut cpu, mut gpu) = (Vec::new(), Vec::new());
    CpuCompositor.composite_into(&screen, 16, 12, Some((&webcam, 24, 8)), cam, &bg, &layout, &scene, &mut cpu);
    g.composite_into(&screen, 16, 12, Some((&webcam, 24, 8)), cam, &bg, &layout, &scene, &mut gpu);
    // Sample OFF-CENTRE, at dest x = 13 (panel-local column 1.5 of 8): with the cover crop that
    // maps to source column 9.5 (green); WITHOUT it, panel-local 0.1875 maps straight to source
    // column 4.5 (BLUE). The panel centre is green either way, so it cannot catch a missing crop.
    for (name, buf) in [("cpu", &cpu), ("gpu", &gpu)] {
        let p = &buf[((11 * 32 + 13) * 4) as usize..][..4];
        assert!(p[1] > 200 && p[0] < 60 && p[2] < 60, "{name} must cover-crop to the middle third, got {p:?}");
    }
    // ...and the two paths must agree across the whole panel interior, not just at one column.
    for x in 13..19u32 {
        let i = ((11 * 32 + x) * 4) as usize;
        for c in 0..4 {
            assert!((cpu[i + c] as i32 - gpu[i + c] as i32).abs() <= 3,
                "CPU/GPU crop mismatch at x={x} ch {c}: {} vs {}", cpu[i + c], gpu[i + c]);
        }
    }
}

#[test]
fn cpu_gpu_parity_two_panels() {
    use crate::export::gpu::compositor::{Compositor, CpuCompositor};
    let g = match GpuCompositor::new(64, 48) { Some(c) => c, None => return };
    let screen = crate::export::color::bgra_to_nv12(&solid(32, 24, [10, 20, 200, 255]), 32, 24);
    let webcam = solid(16, 16, [200, 30, 10, 255]);
    let bg = solid(64, 48, [40, 40, 40, 255]);
    let layout = Layout { out_w: 64, out_h: 48, pad_px: 4, screen_scale: 1.0, screen_radius_px: 48.0 * 0.016 };
    let scene = Scene {
        screen: Panel { rect: RectF { x: 8.0, y: 6.0, w: 30.0, h: 22.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        camera: Panel { rect: RectF { x: 40.0, y: 26.0, w: 18.0, h: 18.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
    };
    let cam = Camera { cx: 32.0, cy: 24.0, scale: 1.0 };
    let mut cpu = Vec::new();
    CpuCompositor.composite_into(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene, &mut cpu);
    let mut gpu = Vec::new();
    g.composite_into(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene, &mut gpu);
    // Interior sample points (centers of bg / screen panel / camera panel) must match within quantization.
    for &(x, y) in &[(2u32, 2u32), (20, 14), (48, 34)] {
        let i = ((y * 64 + x) * 4) as usize;
        for c in 0..4 {
            let d = (cpu[i + c] as i32 - gpu[i + c] as i32).abs();
            // <=3: the screen panel now goes through two independent nv12->RGB converts (CPU
            // u8-rounded `nv12_to_bgra` vs the shader's float math), so allow one extra LSB.
            assert!(d <= 3, "CPU/GPU mismatch at ({x},{y}) ch {c}: {} vs {}", cpu[i + c], gpu[i + c]);
        }
    }
}
