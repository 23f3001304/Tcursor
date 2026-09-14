use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout, RectF};

// All vec2 fields are front-loaded and contiguous so the Rust `[f32; 2]` (size 8)
// offsets match WGSL `vec2<f32>` (align 8). 7 vec2 (56B) + 10 f32 (40B) + 1 vec4 (16B)
// = 112B, a multiple of 16 as the uniform address space requires.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    screen_min: [f32; 2], screen_max: [f32; 2],
    cam_min: [f32; 2], cam_max: [f32; 2],
    zoom_center: [f32; 2],
    // The screen texture sub-rect the screen panel shows, in texture UV (`Scene.src` over the
    // canvas size): (0,0)..(1,1) for an ordinary take, one display switch's fitted rect per span.
    src_min: [f32; 2], src_max: [f32; 2],
    inv_scale: f32,
    screen_r: f32, camera_r: f32,
    screen_a: f32, camera_a: f32,
    screen_w: f32, screen_h: f32,
    cam_w: f32, cam_h: f32,
    wc_aspect: f32, // decoded webcam w/h (was `_pad`) - the shader cover-crops it to the panel
    ring: [f32; 4], // x = ring width (px), yzw = ring color 0..1; x == 0 -> no ring
}

/// Build the uniform: both panel rects (UV), the screen panel's radius/size/alpha,
/// the camera panel's radius/size/alpha, the whole-scene zoom (center + 1/scale),
/// and the optional camera-panel ring (width px + color 0..1) carried on `scene.camera`.
///
/// `webcam` is the DECODED frame's `(w, h)` (`None` = no webcam): one decode box serves every
/// layout, so the shader needs its aspect to cover-crop it to whatever aspect the camera panel
/// has this frame (`shader.wgsl`, mirroring `compositor::cover_rect` on the CPU path). 1.0 when
/// there is no webcam - the panel is not drawn at all then (`camera_a` 0).
///
/// `screen` is the decoded screen frame's `(w, h)`, needed only to normalize `scene.src` into
/// texture UV - the sub-rect of the canvas the screen panel shows (the whole texture unless a
/// mid-take display switch cropped it; see `Scene.src`).
pub fn build_uniforms(scene: &Scene, cam: Camera, layout: &Layout, webcam: Option<(u32, u32)>,
                      screen: (u32, u32)) -> Uniforms {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let uv = |r: RectF| ([r.x / ow, r.y / oh], [(r.x + r.w) / ow, (r.y + r.h) / oh]);
    let (smin, smax) = uv(scene.screen.rect);
    let (cmin, cmax) = uv(scene.camera.rect);
    let (tw, th) = (screen.0.max(1) as f32, screen.1.max(1) as f32);
    let src = scene.src;
    let (src_min, src_max) = ([src.x / tw, src.y / th], [(src.x + src.w) / tw, (src.y + src.h) / th]);
    let ring = if scene.camera.ring_px > 0.0 {
        let [r, g, b] = scene.camera.ring_color;
        [scene.camera.ring_px, r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    } else { [0.0, 0.0, 0.0, 0.0] };
    Uniforms {
        screen_min: smin, screen_max: smax, cam_min: cmin, cam_max: cmax,
        zoom_center: [cam.cx / ow, cam.cy / oh],
        src_min, src_max,
        inv_scale: 1.0 / cam.scale.max(0.01),
        screen_r: scene.screen.radius, camera_r: scene.camera.radius,
        screen_a: scene.screen.alpha,
        camera_a: if webcam.is_some() { scene.camera.alpha } else { 0.0 },
        screen_w: scene.screen.rect.w, screen_h: scene.screen.rect.h,
        cam_w: scene.camera.rect.w, cam_h: scene.camera.rect.h,
        wc_aspect: webcam.map(|(w, h)| w.max(1) as f32 / h.max(1) as f32).unwrap_or(1.0),
        ring,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::scene::Panel;
    use crate::export::types::RectF;

    fn scene_with_ring(ring_px: f32, ring_color: [u8; 3]) -> Scene {
        Scene {
            screen: Panel { rect: RectF { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
            camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, radius: 0.0, alpha: 1.0, ring_px, ring_color },
            src: crate::export::coordmap::full_src(16, 16),
        }
    }

    #[test]
    fn no_ring_is_all_zero() {
        let s = scene_with_ring(0.0, [255, 255, 255]);
        let u = build_uniforms(&s, Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, &Layout::default(), Some((16, 16)), (16, 16));
        assert_eq!(u.ring, [0.0, 0.0, 0.0, 0.0]);
    }

    /// The shader crops the webcam to the panel, so it needs the DECODED frame's aspect - and a
    /// missing webcam must not leave a 0 (or a NaN) in a field the shader divides by.
    #[test]
    fn wc_aspect_is_the_decoded_frames_own_ratio() {
        let s = scene_with_ring(0.0, [0, 0, 0]);
        let (cam, l) = (Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, Layout::default());
        assert!((build_uniforms(&s, cam, &l, Some((1280, 720)), (16, 16)).wc_aspect - 16.0 / 9.0).abs() < 1e-6);
        assert_eq!(build_uniforms(&s, cam, &l, Some((720, 720)), (16, 16)).wc_aspect, 1.0);
        let none = build_uniforms(&s, cam, &l, None, (16, 16));
        assert_eq!((none.wc_aspect, none.camera_a), (1.0, 0.0));
    }

    /// `src` normalizes against the SCREEN TEXTURE's size, not the output frame: the whole canvas
    /// is the full 0..1 texture (a take that never switched display samples exactly as before),
    /// and a 16:10 display fitted into a 1920x1080 canvas is the 96..1824 band.
    #[test]
    fn src_is_the_scene_rect_in_screen_texture_uv() {
        let (cam, l) = (Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, Layout::default());
        let mut s = scene_with_ring(0.0, [0, 0, 0]);
        s.src = crate::export::coordmap::full_src(1920, 1080);
        let u = build_uniforms(&s, cam, &l, None, (1920, 1080));
        assert_eq!((u.src_min, u.src_max), ([0.0, 0.0], [1.0, 1.0]));
        s.src = RectF { x: 96.0, y: 0.0, w: 1728.0, h: 1080.0 };
        let u = build_uniforms(&s, cam, &l, None, (1920, 1080));
        assert!((u.src_min[0] - 96.0 / 1920.0).abs() < 1e-6 && u.src_min[1] == 0.0);
        assert!((u.src_max[0] - 1824.0 / 1920.0).abs() < 1e-6 && u.src_max[1] == 1.0);
    }

    #[test]
    fn ring_packs_width_and_normalized_color() {
        let s = scene_with_ring(6.0, [255, 128, 0]);
        let u = build_uniforms(&s, Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, &Layout::default(), Some((16, 16)), (16, 16));
        assert_eq!(u.ring[0], 6.0);
        assert_eq!(u.ring[1], 1.0);
        assert!((u.ring[2] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(u.ring[3], 0.0);
    }
}
