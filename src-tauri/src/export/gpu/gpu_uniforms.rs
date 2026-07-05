use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout, RectF};

// All vec2 fields are front-loaded and contiguous so the Rust `[f32; 2]` (size 8)
// offsets match WGSL `vec2<f32>` (align 8). 5 vec2 (40B) + 10 f32 (40B) = 80B,
// a multiple of 16 as the uniform address space requires. `_pad` keeps that size.
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
}

/// Build the uniform: both panel rects (UV), the screen panel's radius/size/alpha,
/// the camera panel's radius/size/alpha, and the whole-scene zoom (center + 1/scale).
pub fn build_uniforms(scene: &Scene, cam: Camera, layout: &Layout, has_webcam: bool) -> Uniforms {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let uv = |r: RectF| ([r.x / ow, r.y / oh], [(r.x + r.w) / ow, (r.y + r.h) / oh]);
    let (smin, smax) = uv(scene.screen.rect);
    let (cmin, cmax) = uv(scene.camera.rect);
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
    }
}
