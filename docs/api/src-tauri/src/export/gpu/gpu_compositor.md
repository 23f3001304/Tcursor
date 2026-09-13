# src-tauri/src/export/gpu/gpu_compositor.rs

GPU-accelerated compositor backed by wgpu. Implements the `Compositor` trait by uploading the nv12 screen (as separate Y/UV textures) and the BGRA webcam into a lazily-(re)built resource bundle, running the WGSL compositing shader (which converts nv12 -> RGB via `screen_rgb`), and reading back BGRA pixels to a `Vec<u8>` that matches the `CpuCompositor` output format exactly (verified by the `cpu_gpu_parity_two_panels` test). Its tests live in `gpu_compositor_tests.rs` (`#[path]`-included) so this file stays under the line budget; each no-ops when the machine has no usable wgpu adapter.

## GpuCompositor

```rust
pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
    res: Mutex<Option<CompositorResources>>,
}
```

GPU compositor holding device-lifetime state plus a lazily-(re)built bundle of size-dependent resources.

- `gpu: Gpu` - device, queue, pipeline, output texture, and readback buffer. *Why:* all device-lifetime resources are held here and shared across every `composite_into` call.
- `out_w: u32, out_h: u32` - output frame dimensions in pixels. *Why:* needed to strip row padding during readback and to construct `Uniforms`.
- `res: Mutex<Option<CompositorResources>>` - the current `(sw, sh, ww, wh)` bundle of textures/uniform-buffer/bind-group, built lazily on first use and rebuilt whenever the incoming screen or webcam dimensions change. *Why a `Mutex`, not `OnceLock`:* `Compositor::composite_into` takes `&self` (the trait is shared through `Box<dyn Compositor>`), so caching mutable GPU state needs interior mutability; and unlike the background (fixed at `out_w x out_h` for the whole export), screen/webcam sizes can change mid-lifetime (e.g. a resized preview), so the cache must support being invalidated and rebuilt, not just initialized once.

### CompositorResources (private)

```rust
struct CompositorResources {
    sw: u32, sh: u32, ww: u32, wh: u32,
    screen_y_tex: wgpu::Texture,   // nv12 Y plane (R8, sw x sh)
    screen_uv_tex: wgpu::Texture,  // nv12 interleaved UV (Rg8, sw/2 x sh/2)
    webcam_tex: wgpu::Texture,
    bg_tex: wgpu::Texture,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    bg_key: Option<u64>,
}
```

Everything that depends on frame size, built in one shot by `gpu_compositor_tex::build_resources`. `sw/sh/ww/wh` record the sizes this bundle was built for, so the next `composite_into` call can detect a size change and trigger a rebuild; `bg_key` records the CONTENT key (`gpu_compositor_tex::bg_key`) of the background actually sitting in `bg_tex`, `None` on every (re)build. It replaced a plain `bg_uploaded: bool`, which only ever flipped `false` on a rebuild - and since `FrameRenderer::reload_edit` rebuilds `bg` at the SAME dimensions, no dimension-driven rebuild ever fires for a background edit, so every newly-built background buffer was handed to `composite_into` and thrown away while the shader kept sampling the stale texture.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` builds one via `select_compositor`; `FrameRenderer::composite_at` calls `composite_into` once per frame.

## GpuCompositor::new

```rust
pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor>
```

Constructs a `GpuCompositor` by calling `Gpu::new(out_w, out_h)`. Returns `None` if no GPU adapter is available or device creation fails.

### Inputs

- `out_w: u32, out_h: u32` - output dimensions; forwarded to `Gpu::new` for texture and buffer sizing. *Why:* all GPU resources are pre-allocated to the output size at construction time.

### Returns

`Some(GpuCompositor)` with `res` set to `Mutex::new(None)` - the first resource bundle is built lazily on the first `composite_into` call, not here; `None` if no GPU is found.

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

- `screen, sw, sh` - captured screen pixels in **nv12** (`sw*sh*3/2` bytes: Y plane then interleaved half-res UV) and dimensions. *Why:* uploaded every frame into the cached `screen_y_tex`/`screen_uv_tex` (via `update_tex_bpp`) and converted to RGB in the shader (`screen_rgb`); a change in `sw`/`sh` from the cached bundle is exactly what triggers a resource rebuild.
- `webcam` - optional `(bytes, w, h)` for the webcam. *Why:* when `None`, a 1x1 black placeholder is used and `camera_a` in the uniform is forced to `0.0` so the placeholder is never sampled visibly.
- `cam: Camera` - zoom center and scale. *Why:* packed into the uniform buffer so the shader can compute the zoom crop in UV space.
- `bg: &[u8]` - background pixels. *Why:* uploaded whenever its CONTENT key changes (`bg_key`), not once per compositor lifetime and not only on a size-driven rebuild - so an editor background edit (kind/colour/gradient/blur) reaching the warm preview renderer through `reload_edit` actually appears.
- `layout: &Layout` - output dimensions for UV normalization in `build_uniforms`.
- `scene: &Scene` - both panel rects, radii, and alphas. *Why:* all per-frame compositing parameters are derived from this.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller reuse the same allocation across frames instead of the compositor allocating fresh each call.

### Returns

Nothing (`()`). `out` is resized to `out_w * out_h * 4` bytes and fully overwritten with BGRA pixels, row padding stripped.

### Implementation

1. Unwrap `webcam` or substitute a 1x1 black placeholder `(&[0u8; 4], 1, 1)`.
2. Call `build_uniforms(scene, cam, layout, webcam.map(|(_, w, h)| (w, h)))` up front - the dims travel into the uniform as `wc_aspect` so the shader can cover-crop the single decode box to this frame's panel aspect - needed either to seed a brand-new resource bundle or to refresh the cached one.
3. Lock `res`; `rebuild` is true when there is no cached bundle yet, or its `(sw, sh, ww, wh)` differs from this call's.
4. If `rebuild`, call `gpu_compositor_tex::build_resources(g, sw, sh, ww, wh, ow, oh, &u)` and replace the cached bundle - this allocates fresh Y/UV/webcam/bg textures, a new uniform buffer, and a new bind group, and resets `bg_key` to `None`.
5. Upload the nv12 screen into the (possibly just-rebuilt) bundle: `update_tex_bpp` the Y plane (`screen[..sw*sh]`, `R8`, 1 byte/px) and the interleaved UV plane (`screen[sw*sh..]`, half-res, `Rg8`, 2 bytes/px); `update_tex` the webcam (BGRA, 4 bytes/px).
6. Compute `bg_key(bg)` - unless the background is DYNAMIC (a video/GIF asset; see `Compositor::set_bg_dynamic`), in which case the key is `None` and never computed, because that path re-uploads either way and the hash would be pure cost - and upload the background when `gpu_compositor_tex::should_upload(r.bg_key != key, dynamic)` says so, then store it - so a rebuild re-uploads into the fresh `bg_tex`, AND a same-size background EDIT re-uploads too. The key is a length-seeded FNV-1a over ~4k strided samples: ~0.1-0.3 ms per frame against an ~8 MB buffer (cache-miss bound, not arithmetic), which is why it is computed BEFORE the `res` lock is taken - the work needs nothing from the cached bundle and must not widen the critical section.
7. `write_buffer` this frame's `Uniforms` into `ubuf` unconditionally (the buffer itself is only recreated on rebuild; its contents refresh every frame).
8. Begin a command encoder; start a render pass that clears to black and draws 3 vertices (full-screen triangle - no vertex buffer needed).
9. `copy_texture_to_buffer` from `out_tex` to `readback` using `padded_bpr`.
10. Submit the encoder; map the readback buffer in read mode; poll with `Maintain::Wait` for synchronous completion.
11. Clear `out`. When `padded_bpr` equals `out_w * 4` (no row padding - true whenever the output width is already a multiple of the 256-byte alignment), one contiguous `extend_from_slice`; otherwise resize `out` and copy row-by-row, stripping the padding. Unmap.

### Behaviors worth knowing

- `screen_panel_composites_onto_background` (unit test): same geometry as the CPU test - a 4x4 red screen (built via `bgra_to_nv12`, uploaded as Y/UV textures, converted back by `screen_rgb`) at (2,2) in an 8x8 blue background must leave the corner blue and the panel interior approximately red - within a few LSBs, not bit-exact.
- `cpu_gpu_parity_cover_crops_a_wide_webcam_into_a_square_panel` (unit test): a 3:1 webcam composited into a square panel shows the middle third on BOTH paths - `compositor::cover_rect` and the shader's `cover_uv` must agree, since one decode box now serves every layout.
- `cpu_gpu_parity_two_panels` (unit test): at three sampled points (bg corner, screen panel center, camera panel center) CPU and GPU outputs must agree within 3 quantization steps. Loosened from 2: the screen panel now round-trips through two independent nv12->RGB converts (CPU's u8-rounded `nv12_to_bgra` vs. the shader's float `screen_rgb`), which can differ by one extra LSB.
