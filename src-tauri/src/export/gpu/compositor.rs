use fast_image_resize::{Resizer, ResizeOptions, PixelType};
use fast_image_resize::images::Image;
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, Layout};

pub trait Compositor: Send + Sync {
    fn composite_into(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
        out: &mut Vec<u8>,
    );
}

pub struct CpuCompositor;

impl Compositor for CpuCompositor {
    fn composite_into(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
        out: &mut Vec<u8>,
    ) {
        let (ow, oh) = (layout.out_w, layout.out_h);
        // Fast-path: 1:1 unzoomed full screen without camera PiP or corner rounding
        if cam.scale <= 1.0001 && scene.camera.alpha <= 0.0 && scene.screen.alpha >= 0.999 && scene.screen.radius <= 0.1 && scene.screen.ring_px <= 0.1 && sw == ow && sh == oh && screen.len() == (ow * oh * 4) as usize {
            out.clear();
            out.extend_from_slice(screen);
            return;
        }
        let mut base = bg.to_vec();
        draw_panel(&mut base, ow, oh, screen, sw, sh, scene.screen);
        let (cx0, cy0, cw, ch) = crate::export::coordmap::crop(cam, ow, oh);
        let resized = resize_crop(&base, ow, oh, cx0 as f64, cy0 as f64, cw as f64, ch as f64, ow, oh);
        out.clear();
        out.extend_from_slice(&resized);
        if let Some((wc, ww, wh)) = webcam {
            draw_panel(out, ow, oh, wc, ww, wh, scene.camera);
        }
    }
}

/// GPU compositor if available, else CPU fallback. Never fails.
pub fn select_compositor(layout: &Layout) -> Box<dyn Compositor> {
    if let Some(c) = crate::export::gpu::gpu_compositor::GpuCompositor::new(layout.out_w, layout.out_h) {
        return Box::new(c);
    }
    Box::new(CpuCompositor)
}

/// Rounded-box signed distance (px) at pixel center `(tx+0.5, ty+0.5)` inside a `pw`x`ph`
/// panel with corner radius `r`. Negative inside, 0 at the edge, positive outside -
/// identical formula to the WGSL shader's `rrect_sd` (see gpu/shader.wgsl).
fn rrect_sd_px(tx: u32, ty: u32, pw: u32, ph: u32, r: f32) -> f32 {
    let (hw, hh) = (pw as f32 / 2.0, ph as f32 / 2.0);
    let qx = ((tx as f32 + 0.5) - hw).abs() - (hw - r);
    let qy = ((ty as f32 + 0.5) - hh).abs() - (hh - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    qx.max(qy).min(0.0) + outside - r
}

/// Resize `src` (sw×sh) into `panel.rect` and blend onto `dst` with rounded-rect
/// antialiased coverage times `panel.alpha`. No-op when invisible or degenerate.
/// When `panel.ring_px > 0`, also blends a ring/border band just inside the panel
/// edge (mirrors the GPU shader's post-camera-mix ring blend, same SDF/band formula).
fn draw_panel(dst: &mut [u8], dw: u32, dh: u32, src: &[u8], sw: u32, sh: u32, panel: Panel) {
    if panel.alpha <= 0.0 { return; }
    let (pw, ph) = (panel.rect.w.round() as u32, panel.rect.h.round() as u32);
    if pw == 0 || ph == 0 { return; }
    let resized = resize_crop(src, sw, sh, 0.0, 0.0, sw as f64, sh as f64, pw, ph);
    let r = panel.radius.clamp(0.0, pw.min(ph) as f32 / 2.0);
    let a = panel.alpha.clamp(0.0, 1.0);
    // For a fully-opaque panel, pixels safely inside the rounded corners always
    // have coverage == 1.0 (see blit's opaque_inner doc); skip the sqrt SDF there.
    let inner = if a >= 1.0 {
        let ins = (r.ceil() as u32).saturating_add(2);
        (pw > 2 * ins && ph > 2 * ins).then(|| (ins, ins, pw - ins, ph - ins))
    } else { None };
    let cov = move |tx: u32, ty: u32| {
        let d = rrect_sd_px(tx, ty, pw, ph, r); // rounded-box SDF (== shader)
        (0.5 - d).clamp(0.0, 1.0) * a
    };
    let ox = panel.rect.x.max(0.0).round() as u32;
    let oy = panel.rect.y.max(0.0).round() as u32;
    blit(dst, dw, dh, &resized, pw, ph, ox, oy, inner, cov);
    if panel.ring_px > 0.0 {
        blit_ring(dst, dw, dh, pw, ph, ox, oy, r, panel.ring_px, panel.ring_color, a);
    }
}

/// Blend `ring_color` over `dst` in a band just inside the panel edge, width `ring_px`,
/// weighted by panel alpha `a`. Same SDF/band math as the WGSL shader's ring blend.
fn blit_ring(dst: &mut [u8], dw: u32, dh: u32, pw: u32, ph: u32, ox: u32, oy: u32,
             r: f32, ring_px: f32, ring_color: [u8; 3], a: f32) {
    let [rr, rg, rb] = ring_color;
    for ty in 0..ph {
        for tx in 0..pw {
            let d = rrect_sd_px(tx, ty, pw, ph, r);
            if d > 0.0 || d < -ring_px { continue; } // outside the panel or inside the ring band
            let band = ((ring_px + d) / ring_px.max(1.0)).clamp(0.0, 1.0) * a;
            if band <= 0.0 { continue; }
            let (dx, dy) = (ox + tx, oy + ty);
            if dx >= dw || dy >= dh { continue; }
            let di = ((dy * dw + dx) * 4) as usize;
            // BGRA destination bytes; ring_color is RGB.
            dst[di] = (rb as f32 * band + dst[di] as f32 * (1.0 - band)).round() as u8;
            dst[di + 1] = (rg as f32 * band + dst[di + 1] as f32 * (1.0 - band)).round() as u8;
            dst[di + 2] = (rr as f32 * band + dst[di + 2] as f32 * (1.0 - band)).round() as u8;
        }
    }
}

fn resize_crop(
    src: &[u8], sw: u32, sh: u32,
    cx: f64, cy: f64, cw: f64, ch: f64,
    dst_w: u32, dst_h: u32,
) -> Vec<u8> {
    let src_img = Image::from_vec_u8(sw, sh, src.to_vec(), PixelType::U8x4).expect("resize_crop: invalid src");
    let mut dst_img = Image::new(dst_w, dst_h, PixelType::U8x4);
    let mut r = Resizer::new();
    r.resize(&src_img, &mut dst_img, &ResizeOptions::new().crop(cx, cy, cw, ch)).expect("resize_crop: resize failed");
    dst_img.into_vec()
}

// Blend `src` onto `dst` at (ox,oy); `alpha` is per-pixel coverage in [0,1].
// Destination-bounds checked.
fn blit(
    dst: &mut [u8], dst_w: u32, dst_h: u32,
    src: &[u8], src_w: u32, src_h: u32,
    ox: u32, oy: u32,
    opaque_inner: Option<(u32, u32, u32, u32)>, // (x0,y0,x1,y1) where coverage is exactly 1.0
    alpha: impl Fn(u32, u32) -> f32,
) {
    for ty in 0..src_h {
        for tx in 0..src_w {
            let inside = matches!(opaque_inner, Some((x0, y0, x1, y1)) if tx >= x0 && tx < x1 && ty >= y0 && ty < y1);
            let a = if inside { 1.0 } else { alpha(tx, ty) }; // skip the sqrt SDF for guaranteed-opaque pixels
            if a <= 0.0 { continue; }
            let (dx, dy) = (ox + tx, oy + ty);
            if dx >= dst_w || dy >= dst_h { continue; }
            let si = ((ty * src_w + tx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;
            if a >= 1.0 {
                dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
            } else {
                for c in 0..4 {
                    dst[di + c] = (src[si + c] as f32 * a + dst[di + c] as f32 * (1.0 - a)).round() as u8;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "compositor_tests.rs"]
mod tests;
