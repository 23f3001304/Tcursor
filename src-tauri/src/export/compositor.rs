use fast_image_resize::{Resizer, ResizeOptions, PixelType};
use fast_image_resize::images::Image;
use crate::export::types::{Camera, Layout, OverlayLayout, OverlayPos, OverlayShape};

pub trait Compositor: Send + Sync {
    fn composite(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        overlay: &OverlayLayout,
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
        overlay: &OverlayLayout,
    ) -> Vec<u8> {
        let out_w = layout.out_w;
        let out_h = layout.out_h;
        let pad = layout.pad_px;
        let mut out = bg.to_vec();

        // Crop rect from camera
        let cw = (sw as f32 / cam.scale).round() as u32;
        let ch = (sh as f32 / cam.scale).round() as u32;
        let cx0 = (cam.cx - cw as f32 / 2.0).clamp(0.0, (sw - cw) as f32) as f64;
        let cy0 = (cam.cy - ch as f32 / 2.0).clamp(0.0, (sh - ch) as f32) as f64;

        // Fix 1: use saturating_sub to guard against degenerate pad >= out dims
        let dst_w = out_w.saturating_sub(2 * pad);
        let dst_h = out_h.saturating_sub(2 * pad);
        if dst_w > 0 && dst_h > 0 {
            let resized = resize_crop(screen, sw, sh, cx0, cy0, cw as f64, ch as f64, dst_w, dst_h);
            blit(&mut out, out_w, out_h, &resized, dst_w, dst_h, pad, pad, |_, _| true);
        }

        if overlay.enabled {
            if let Some((wc, ww, wh)) = webcam {
                let sz = overlay.size_px;
                let m = overlay.margin_px;
                let wc_resized = resize_crop(wc, ww, wh, 0.0, 0.0, ww as f64, wh as f64, sz, sz);
                // Fix 2: overlay_origin uses saturating_sub
                let (ox, oy) = overlay_origin(&overlay.pos, m, sz, out_w, out_h);
                let half = sz as f32 / 2.0;
                let mask: Box<dyn Fn(u32, u32) -> bool> = match overlay.shape {
                    OverlayShape::Circle => Box::new(move |tx, ty| {
                        let dx = tx as f32 - half + 0.5;
                        let dy = ty as f32 - half + 0.5;
                        (dx * dx + dy * dy).sqrt() <= half
                    }),
                    _ => Box::new(|_, _| true),
                };
                // Fix 2: blit now receives dst_h for edge clamping
                blit(&mut out, out_w, out_h, &wc_resized, sz, sz, ox, oy, |tx, ty| mask(tx, ty));
            }
        }

        out
    }
}

fn resize_crop(
    src: &[u8], sw: u32, sh: u32,
    cx: f64, cy: f64, cw: f64, ch: f64,
    dst_w: u32, dst_h: u32,
) -> Vec<u8> {
    let src_img = Image::from_vec_u8(sw, sh, src.to_vec(), PixelType::U8x4)
        .expect("resize_crop: invalid src");
    let mut dst_img = Image::new(dst_w, dst_h, PixelType::U8x4);
    let mut r = Resizer::new();
    r.resize(&src_img, &mut dst_img, &ResizeOptions::new().crop(cx, cy, cw, ch))
        .expect("resize_crop: resize failed");
    dst_img.into_vec()
}

// Fix 2: accept dst_h to clamp destination coords before indexing
fn blit(
    dst: &mut [u8], dst_w: u32, dst_h: u32,
    src: &[u8], src_w: u32, src_h: u32,
    ox: u32, oy: u32,
    mask: impl Fn(u32, u32) -> bool,
) {
    for ty in 0..src_h {
        for tx in 0..src_w {
            if !mask(tx, ty) { continue; }
            // Skip pixels whose destination coords are out of bounds
            let dx = ox + tx;
            let dy = oy + ty;
            if dx >= dst_w || dy >= dst_h { continue; }
            let si = ((ty * src_w + tx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
}

// Fix 2: use saturating_sub so an oversized tile clamps to origin 0
fn overlay_origin(pos: &OverlayPos, m: u32, sz: u32, out_w: u32, out_h: u32) -> (u32, u32) {
    match pos {
        OverlayPos::BottomLeft  => (m, out_h.saturating_sub(sz + m)),
        OverlayPos::BottomRight => (out_w.saturating_sub(sz + m), out_h.saturating_sub(sz + m)),
        OverlayPos::TopLeft     => (m, m),
        OverlayPos::TopRight    => (out_w.saturating_sub(sz + m), m),
        OverlayPos::Custom { x, y } => (*x, *y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::types::{Camera, Layout, OverlayLayout, OverlayPos, OverlayShape};

    #[test]
    fn composites_onto_background_at_inset() {
        // 4x4 red screen, 8x8 output, pad 1, no zoom, no webcam, blue bg.
        let sw = 4u32; let sh = 4u32;
        let screen = solid(sw, sh, [0, 0, 255, 255]); // red (BGRA)
        let bg = solid(8, 8, [255, 0, 0, 255]);        // blue
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1 };
        let overlay = OverlayLayout { enabled: false, ..OverlayLayout::default() };
        let cam = Camera { cx: 2.0, cy: 2.0, scale: 1.0 };
        let out = CpuCompositor.composite(&screen, sw, sh, None, cam, &bg, &layout, &overlay);
        assert_eq!(out.len(), 8 * 8 * 4);
        // corner pixel (0,0) is background (blue); an inset pixel (e.g. 3,3) is screen (red)
        assert_eq!(&out[0..4], &[255, 0, 0, 255]);
        let i = ((3 * 8 + 3) * 4) as usize;
        assert_eq!(&out[i..i + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn oversized_overlay_and_degenerate_pad_do_not_panic() {
        // overlay tile (size_px=20) larger than output (8x8) — would OOB-index before fix
        let sw = 8u32; let sh = 8u32;
        let screen = solid(sw, sh, [0, 0, 255, 255]);
        let webcam = solid(4, 4, [0, 255, 0, 255]);
        let bg = solid(8, 8, [255, 0, 0, 255]);
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1 };
        let overlay = OverlayLayout {
            enabled: true,
            size_px: 20,
            margin_px: 0,
            pos: OverlayPos::BottomRight,
            shape: OverlayShape::Circle,
        };
        let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
        let out = CpuCompositor.composite(&screen, sw, sh, Some((&webcam, 4, 4)), cam, &bg, &layout, &overlay);
        assert_eq!(out.len(), 8 * 8 * 4, "output buffer must be exactly out_w*out_h*4");

        // degenerate pad: pad_px so large that 2*pad >= out_w => dst_w == 0; must not panic
        let layout_huge_pad = Layout { out_w: 8, out_h: 8, pad_px: 10 };
        let overlay_off = OverlayLayout { enabled: false, ..OverlayLayout::default() };
        let out2 = CpuCompositor.composite(&screen, sw, sh, None, cam, &bg, &layout_huge_pad, &overlay_off);
        assert_eq!(out2.len(), 8 * 8 * 4, "degenerate pad: output must still be out_w*out_h*4");
    }

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for c in v.chunks_mut(4) { c.copy_from_slice(&px); } v
    }
}
