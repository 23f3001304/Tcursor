use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout, RectF};

// All vec2 fields are front-loaded and contiguous so the Rust `[f32; 2]` (size 8)
// offsets match WGSL `vec2<f32>` (align 8). 5 vec2 (40B) + 10 f32 (40B) + 1 vec4 (16B)
// = 96B, a multiple of 16 as the uniform address space requires. `_pad` keeps the
// f32 block's own size at a multiple of 8 (unchanged since before `ring` was added).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    screen_min: [f32; 2], screen_max: [f32; 2],
    cam_min: [f32; 2], cam_max: [f32; 2],
    zoom_center: [f32; 2],
    inv_scale: f32,
    screen_r: f32, camera_r: f32,
    screen_a: f32, camera_a: f32,
    screen_w: f32, screen_h: f32,
    cam_w: f32, cam_h: f32,
    _pad: f32,
    ring: [f32; 4], // x = ring width (px), yzw = ring color 0..1; x == 0 -> no ring
}

/// Build the uniform: both panel rects (UV), the screen panel's radius/size/alpha,
/// the camera panel's radius/size/alpha, the whole-scene zoom (center + 1/scale),
/// and the optional camera-panel ring (width px + color 0..1) carried on `scene.camera`.
pub fn build_uniforms(scene: &Scene, cam: Camera, layout: &Layout, has_webcam: bool) -> Uniforms {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let uv = |r: RectF| ([r.x / ow, r.y / oh], [(r.x + r.w) / ow, (r.y + r.h) / oh]);
    let (smin, smax) = uv(scene.screen.rect);
    let (cmin, cmax) = uv(scene.camera.rect);
    let ring = if scene.camera.ring_px > 0.0 {
        let [r, g, b] = scene.camera.ring_color;
        [scene.camera.ring_px, r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
    } else { [0.0, 0.0, 0.0, 0.0] };
    Uniforms {
        screen_min: smin, screen_max: smax, cam_min: cmin, cam_max: cmax,
        zoom_center: [cam.cx / ow, cam.cy / oh],
        inv_scale: 1.0 / cam.scale.max(0.01),
        screen_r: scene.screen.radius, camera_r: scene.camera.radius,
        screen_a: scene.screen.alpha,
        camera_a: if has_webcam { scene.camera.alpha } else { 0.0 },
        screen_w: scene.screen.rect.w, screen_h: scene.screen.rect.h,
        cam_w: scene.camera.rect.w, cam_h: scene.camera.rect.h,
        _pad: 0.0,
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
        }
    }

    #[test]
    fn no_ring_is_all_zero() {
        let s = scene_with_ring(0.0, [255, 255, 255]);
        let u = build_uniforms(&s, Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, &Layout::default(), true);
        assert_eq!(u.ring, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn ring_packs_width_and_normalized_color() {
        let s = scene_with_ring(6.0, [255, 128, 0]);
        let u = build_uniforms(&s, Camera { cx: 0.0, cy: 0.0, scale: 1.0 }, &Layout::default(), true);
        assert_eq!(u.ring[0], 6.0);
        assert_eq!(u.ring[1], 1.0);
        assert!((u.ring[2] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(u.ring[3], 0.0);
    }
}
