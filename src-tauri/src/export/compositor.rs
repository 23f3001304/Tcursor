use fast_image_resize::{Resizer, ResizeOptions, PixelType};
use fast_image_resize::images::Image;
use crate::export::scene::{Panel, Scene};
use crate::export::types::{Camera, Layout};

pub trait Compositor: Send + Sync {
    fn composite(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
    ) -> Vec<u8>;
}

pub struct CpuCompositor;

impl Compositor for CpuCompositor {
    fn composite(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
    ) -> Vec<u8> {
        let (ow, oh) = (layout.out_w, layout.out_h);
        // 1. Base scene: background + the screen panel (rounded rect, alpha).
        let mut base = bg.to_vec();
        draw_panel(&mut base, ow, oh, screen, sw, sh, scene.screen);
        // 2. Zoom the WHOLE base scene toward the camera center (output coords).
        let (cx0, cy0, cw, ch) = crate::export::coordmap::crop(cam, ow, oh);
        let mut out = resize_crop(&base, ow, oh, cx0 as f64, cy0 as f64, cw as f64, ch as f64, ow, oh);
        // 3. Camera panel on top (fixed; not zoomed).
        if let Some((wc, ww, wh)) = webcam {
            draw_panel(&mut out, ow, oh, wc, ww, wh, scene.camera);
        }
        out
    }
}

/// Resize `src` (sw×sh) into `panel.rect` and blend onto `dst` with rounded-rect
/// antialiased coverage times `panel.alpha`. No-op when invisible or degenerate.
fn draw_panel(dst: &mut [u8], dw: u32, dh: u32, src: &[u8], sw: u32, sh: u32, panel: Panel) {
    if panel.alpha <= 0.0 { return; }
    let (pw, ph) = (panel.rect.w.round() as u32, panel.rect.h.round() as u32);
    if pw == 0 || ph == 0 { return; }
    let resized = resize_crop(src, sw, sh, 0.0, 0.0, sw as f64, sh as f64, pw, ph);
    let r = panel.radius.clamp(0.0, pw.min(ph) as f32 / 2.0);
    let (hw, hh) = (pw as f32 / 2.0, ph as f32 / 2.0);
    let a = panel.alpha.clamp(0.0, 1.0);
    // For a fully-opaque panel, pixels safely inside the rounded corners always
    // have coverage == 1.0 (see blit's opaque_inner doc); skip the sqrt SDF there.
    let inner = if a >= 1.0 {
        let ins = (r.ceil() as u32).saturating_add(2);
        (pw > 2 * ins && ph > 2 * ins).then(|| (ins, ins, pw - ins, ph - ins))
    } else { None };
    let cov = move |tx: u32, ty: u32| {
        let qx = ((tx as f32 + 0.5) - hw).abs() - (hw - r);
        let qy = ((ty as f32 + 0.5) - hh).abs() - (hh - r);
        let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
        let d = qx.max(qy).min(0.0) + outside - r; // rounded-box SDF (== shader)
        (0.5 - d).clamp(0.0, 1.0) * a
    };
    let ox = panel.rect.x.max(0.0).round() as u32;
    let oy = panel.rect.y.max(0.0).round() as u32;
    blit(dst, dw, dh, &resized, pw, ph, ox, oy, inner, cov);
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
mod tests {
    use super::*;
    use crate::export::scene::{Panel, Scene};
    use crate::export::types::{Camera, Layout, RectF};

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for c in v.chunks_mut(4) { c.copy_from_slice(&px); } v
    }
    fn panel(x: f32, y: f32, w: f32, h: f32, a: f32) -> Panel {
        Panel { rect: RectF { x, y, w, h }, radius: 0.0, alpha: a }
    }

    #[test]
    fn screen_panel_composites_onto_background() {
        // 4x4 red screen into a 4x4 panel at (2,2) of an 8x8 blue output, no zoom, no camera.
        let screen = solid(4, 4, [0, 0, 255, 255]); // BGRA red
        let bg = solid(8, 8, [255, 0, 0, 255]);      // BGRA blue
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
        let scene = Scene { screen: panel(2.0, 2.0, 4.0, 4.0, 1.0), camera: panel(0.0, 0.0, 0.0, 0.0, 0.0) };
        let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
        let out = CpuCompositor.composite(&screen, 4, 4, None, cam, &bg, &layout, &scene);
        assert_eq!(out.len(), 8 * 8 * 4);
        assert_eq!(&out[0..4], &[255, 0, 0, 255]);              // corner = bg blue
        let i = ((3 * 8 + 3) * 4) as usize;
        assert_eq!(&out[i..i + 4], &[0, 0, 255, 255]);          // panel interior = screen red
    }

    #[test]
    fn disabled_and_degenerate_panels_do_not_panic() {
        let screen = solid(8, 8, [0, 0, 255, 255]);
        let webcam = solid(4, 4, [0, 255, 0, 255]);
        let bg = solid(8, 8, [255, 0, 0, 255]);
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
        // camera panel larger than output + screen disabled: must not OOB or panic.
        let scene = Scene { screen: panel(0.0, 0.0, 8.0, 8.0, 0.0), camera: panel(2.0, 2.0, 20.0, 20.0, 1.0) };
        let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
        let out = CpuCompositor.composite(&screen, 8, 8, Some((&webcam, 4, 4)), cam, &bg, &layout, &scene);
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
}
