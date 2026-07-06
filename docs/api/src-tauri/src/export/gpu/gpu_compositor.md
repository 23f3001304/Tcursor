# src-tauri/src/export/gpu/gpu_compositor.rs

GPU-accelerated compositor backed by wgpu. Implements the `Compositor` trait by uploading per-frame textures, running the WGSL compositing shader, and reading back BGRA pixels to a `Vec<u8>` that matches the `CpuCompositor` output format exactly (verified by the `cpu_gpu_parity` test).

## GpuCompositor

```rust
pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
    bg_tex: std::sync::OnceLock<wgpu::Texture>,
}
```

GPU compositor holding device-lifetime state and a lazily-initialized background texture.

- `gpu: Gpu` - device, queue, pipeline, output texture, and readback buffer. *Why:* all device-lifetime resources are held here and shared across every `composite_into` call.
- `out_w: u32, out_h: u32` - output frame dimensions in pixels. *Why:* needed to strip row padding during readback and to construct `Uniforms`.
- `bg_tex: OnceLock<wgpu::Texture>` - the background texture is uploaded once on the first `composite_into` call and reused for the rest of the export. *Why:* the background is constant per export; re-uploading it each frame wastes bandwidth and time.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - `select_compositor` returns a `Box<dyn Compositor>` holding a `GpuCompositor` when the GPU is available.

## GpuCompositor::new

```rust
pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor>
```

Constructs a `GpuCompositor` by calling `Gpu::new(out_w, out_h)`. Returns `None` if no GPU adapter is available or device creation fails.

### Inputs

- `out_w: u32, out_h: u32` - output dimensions; forwarded to `Gpu::new` for texture and buffer sizing. *Why:* all GPU resources are pre-allocated to the output size at construction time.

### Returns

`Some(GpuCompositor)` with an empty `bg_tex` lock; `None` if no GPU is found.

## GpuCompositor::composite_into

```rust
fn composite_into(
    &self,
    screen: &[u8], sw: u32, sh: u32,
    webcam: Option<(&[u8], u32, u32)>,
    cam: Camera, bg: &[u8],
    layout: &Layout,
    scene: &Scene,
    out: &mut Vec<u8>,
)
```

Composites one frame entirely on the GPU and writes BGRA pixels of size `out_w * out_h * 4` into `out`.

### Inputs

- `screen, sw, sh` - captured screen pixels and dimensions. *Why:* uploaded as a fresh `sw x sh` texture each frame; size varies between recordings.
- `webcam` - optional `(bytes, w, h)` for the webcam. *Why:* when `None`, a 1x1 black placeholder is used and `camera_a` in the uniform is forced to `0.0` so the placeholder is never sampled visibly.
- `cam: Camera` - zoom center and scale. *Why:* packed into the uniform buffer so the shader can compute the zoom crop in UV space.
- `bg: &[u8]` - background pixels. *Why:* uploaded via `OnceLock` on the first call only; subsequent calls reuse the cached texture.
- `layout: &Layout` - output dimensions for UV normalization in `build_uniforms`.
- `scene: &Scene` - both panel rects, radii, and alphas. *Why:* all per-frame compositing parameters are derived from this.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller reuse the same allocation across frames instead of the compositor allocating fresh each call.

### Returns

Nothing (`()`). `out` is resized to `out_w * out_h * 4` bytes and fully overwritten with BGRA pixels, row padding stripped.

### Implementation

1. Upload `screen` as a fresh `sw x sh` texture via `gpu.upload_tex`.
2. Get or initialize `bg_tex` from `OnceLock`; upload on first call only.
3. Unwrap `webcam` or substitute a 1x1 black placeholder; upload as a texture.
4. Call `build_uniforms(scene, cam, layout, webcam.is_some())` to build the `Uniforms` struct; upload as a UNIFORM buffer via `create_buffer_init`.
5. Create a bind group: binding 0=bg, 1=screen, 2=webcam, 3=sampler, 4=uniform buffer.
6. Begin a command encoder; start a render pass that clears to black and draws 3 vertices (full-screen triangle - no vertex buffer needed).
7. `copy_texture_to_buffer` from `out_tex` to `readback` using `padded_bpr`.
8. Submit the encoder; poll with `Maintain::Wait` for synchronous completion.
9. Map the readback buffer in read mode; clear and resize `out`; strip row padding row-by-row (`padded_bpr` -> `out_w * 4`) directly into `out`; unmap.

### Behaviors worth knowing

- `screen_panel_composites_onto_background` (unit test): same geometry as the CPU test - a 4x4 red screen at (2,2) in an 8x8 blue background must leave the corner blue and the panel interior red.
- `cpu_gpu_parity_two_panels` (unit test): at three sampled points (bg corner, screen panel center, camera panel center) CPU and GPU outputs must agree within 2 quantization steps (rounding differences only).
