# src-tauri/src/export/gpu/gpu_compositor_tex.rs

Texture and bind-group construction for `GpuCompositor::composite_into`, split out of `gpu_compositor.rs` so that file stays under the size limit. Pure builder - identical to the inline block it replaced when it was factored out, no behavior change of its own.

## build_resources

```rust
pub(super) fn build_resources(
    g: &Gpu, sw: u32, sh: u32, ww: u32, wh: u32, ow: u32, oh: u32, u: &Uniforms,
) -> CompositorResources
```

Builds every texture, the uniform buffer, and the bind group for one `(sw, sh, ww, wh)` size combination - everything `GpuCompositor::composite_into` needs to render a frame at that screen/webcam size. Called only when `composite_into` detects a size change from its cached `CompositorResources` (or on the very first frame, when there is no cached bundle yet).

### Inputs

- `g: &Gpu` - the shared device/queue/pipeline/sampler/bind-group-layout. *Why:* every texture, buffer, and the bind group are created against `g.device`; the pipeline's bind-group layout (`g.bind_layout`, from `gpu_pipeline::make_bind_layout`) fixes the six binding slots used below.
- `sw: u32, sh: u32` - screen frame dimensions. *Why:* size the nv12 Y texture (`sw x sh`, `R8Unorm`) and UV texture (`(sw/2).max(1) x (sh/2).max(1)`, `Rg8Unorm`).
- `ww: u32, wh: u32` - webcam frame dimensions. *Why:* size the webcam texture; the caller passes `(1, 1)` when there is no real webcam frame this size combination.
- `ow: u32, oh: u32` - output frame dimensions. *Why:* size the background texture, which is always exactly the output size regardless of screen/webcam size.
- `u: &Uniforms` - the current frame's uniform values. *Why:* the uniform buffer is created via `create_buffer_init`, so it needs initial contents at construction time - `composite_into` overwrites them with `write_buffer` on every later frame at this same size, but the very first frame at a new size has no buffer to write into yet.

### Returns

A `CompositorResources` with `bg_uploaded: false`. *Why:* the caller (`composite_into`) is responsible for uploading the background into the freshly-created `bg_tex` on the same call that triggered the rebuild; returning the bundle with the flag already `false` is what makes that upload happen exactly once per rebuild.

### Implementation

1. Create `screen_y_tex` (`R8Unorm`, `sw x sh`) and `screen_uv_tex` (`Rg8Unorm`, `(sw/2).max(1) x (sh/2).max(1)`) via `Gpu::create_tex_fmt`.
2. Create `webcam_tex` and `bg_tex` (both `FORMAT`, i.e. BGRA) via `Gpu::create_tex`.
3. Create the uniform buffer `ubuf` via `wgpu::util::DeviceExt::create_buffer_init`, seeded with `bytemuck::bytes_of(u)`, usage `UNIFORM | COPY_DST`.
4. Create a texture view for each of the four textures (Y, UV, bg, webcam).
5. Build the bind group against `g.bind_layout`: binding 0 = bg view, 1 = screen Y view, 2 = screen UV view, 3 = webcam view, 4 = `g.sampler`, 5 = `ubuf.as_entire_binding()`.
6. Return `CompositorResources { sw, sh, ww, wh, screen_y_tex, screen_uv_tex, webcam_tex, bg_tex, ubuf, bind, bg_uploaded: false }`.

### Used by

- `src-tauri/src/export/gpu/gpu_compositor.rs` - `GpuCompositor::composite_into` calls this exactly when `(sw, sh, ww, wh)` differs from the cached `CompositorResources` (or there is no cached bundle yet).
