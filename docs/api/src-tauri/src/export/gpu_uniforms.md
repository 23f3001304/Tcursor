# src-tauri/src/export/gpu_uniforms.rs

Defines the GPU uniform buffer layout (`Uniforms`) and the builder function that populates it from per-frame scene, camera, and layout state. The struct is designed for direct `bytemuck` byte-casting with no padding surprises.

## Uniforms

```rust
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
```

Packed uniform struct sent to the WGSL compositing shader. All coordinates are UV (0..1 over the output canvas) except radii and sizes which are in output pixels.

- `screen_min: [f32; 2]`, `screen_max: [f32; 2]` - UV bounding box of the screen panel. *Why:* the shader uses these to locate and sample the screen texture.
- `cam_min: [f32; 2]`, `cam_max: [f32; 2]` - UV bounding box of the camera (webcam) panel. *Why:* same - locates and clips the webcam texture.
- `zoom_center: [f32; 2]` - UV position of the camera zoom center (`cam.cx / ow`, `cam.cy / oh`). *Why:* the shader computes the zoom crop offset from this.
- `inv_scale: f32` - reciprocal of `cam.scale`, clamped so `scale >= 0.01`. *Why:* the shader multiplies by `inv_scale` to get the fractional crop size; reciprocal avoids a per-fragment divide.
- `screen_r: f32`, `camera_r: f32` - corner radii in output pixels (not UV). *Why:* the rounded-box SDF in the shader operates in pixel space after unprojecting the UV.
- `screen_a: f32`, `camera_a: f32` - panel alphas in 0..1. *Why:* drives blend weight; `camera_a` is forced to `0.0` when no webcam is present.
- `screen_w: f32`, `screen_h: f32`, `cam_w: f32`, `cam_h: f32` - panel sizes in output pixels. *Why:* the SDF needs the panel dimensions in the same space as the radius.
- `_pad: f32` - always `0.0`. *Why:* the 5 `vec2` (40 bytes) + 10 `f32` (40 bytes) total 80 bytes, a multiple of 16 as the uniform address space requires; `_pad` achieves that without needing an extra explicit array.

### Used by

- `src-tauri/src/export/gpu_compositor.rs` - `GpuCompositor::composite_into` calls `build_uniforms` and uploads the result as binding 4.

## build_uniforms

```rust
pub fn build_uniforms(scene: &Scene, cam: Camera, layout: &Layout, has_webcam: bool) -> Uniforms
```

Constructs a `Uniforms` value from the current frame's compositing state.

### Inputs

- `scene: &Scene` - both panels (rects in output pixels, radii, alphas). *Why:* all panel-specific fields are derived from here.
- `cam: Camera` - virtual camera center and scale. *Why:* provides `zoom_center` and `inv_scale`.
- `layout: &Layout` - output dimensions (`out_w`, `out_h`). *Why:* used to normalize output-pixel coordinates to UV.
- `has_webcam: bool` - whether a real webcam frame is present. *Why:* when `false`, `camera_a` is forced to `0.0` so the 1x1 black placeholder texture is never blended into the output.

### Returns

A fully-populated `Uniforms` ready for `bytemuck::bytes_of` and `create_buffer_init`.

### Implementation

1. Convert both panel rects from output pixels to UV using a local closure `uv(r) -> (min, max)`.
2. Set `zoom_center = [cam.cx / ow, cam.cy / oh]`; `inv_scale = 1.0 / cam.scale.max(0.01)`.
3. Copy radii, sizes, and alphas directly; override `camera_a = 0.0` when `!has_webcam`.
4. Set `_pad = 0.0`.
