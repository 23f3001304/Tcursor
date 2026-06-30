# src-tauri/src/export/gpu.rs

wgpu device/pipeline initialization, GPU availability probe, texture upload helper, and the shared `FORMAT` constant for the export GPU compositor. All allocation happens at construction time in `Gpu::new`; per-frame paths touch only `upload_tex` and the already-allocated device/queue.

## align_up

```rust
pub fn align_up(v: u32, align: u32) -> u32
```

Rounds `v` up to the nearest multiple of `align`.

### Inputs

- `v: u32` - the value to align. *Why:* `bytes_per_row` in `copy_texture_to_buffer` must be a multiple of `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (256).
- `align: u32` - the alignment requirement. *Why:* passed as a parameter so the function is reusable; callers always supply `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`.

### Returns

The smallest multiple of `align` >= `v`. Panics if `align == 0` (unsigned wraparound).

### Used by

- `src-tauri/src/export/gpu.rs` - `Gpu::new` uses it to compute `padded_bpr`.

## gpu_available

```rust
pub fn gpu_available() -> bool
```

Returns `true` if wgpu can find at least one adapter on this machine. Blocks the calling thread via `pollster::block_on`.

### Returns

`true` if a default-options adapter was found; `false` otherwise.

### Used by

- `src-tauri/src/export/exporter.rs` - `select_compositor` calls this before attempting `GpuCompositor::new`.

## FORMAT

```rust
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
```

The texture format used for all export textures (background, screen, webcam, output render target). `Bgra8Unorm` keeps channel order matching `CpuCompositor`'s BGRA layout so readback bytes require no channel swap and both paths produce identical pixel data.

### Used by

- `src-tauri/src/export/gpu.rs` - `Gpu::new` and `Gpu::upload_tex` set this format on every texture.
- `src-tauri/src/export/gpu_compositor.rs` - implicitly via the bind group and render pass.

## Gpu

```rust
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sampler: wgpu::Sampler,
    pub bind_layout: wgpu::BindGroupLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub out_tex: wgpu::Texture,
    pub out_view: wgpu::TextureView,
    pub readback: wgpu::Buffer,
    pub padded_bpr: u32,
}
```

All device-lifetime GPU state shared across every frame in one export run.

- `device: wgpu::Device` - the wgpu logical device. *Why:* all resource creation (buffers, textures, bind groups) goes through this.
- `queue: wgpu::Queue` - the command submission queue. *Why:* `submit` and texture uploads are dispatched here.
- `sampler: wgpu::Sampler` - bilinear/linear filter, clamp-to-edge on all axes. *Why:* shared sampler avoids creating one per frame; all texture lookups in the shader use the same filter settings.
- `bind_layout: wgpu::BindGroupLayout` - five entries: bindings 0-2 are 2D float-filterable textures (bg, screen, webcam); binding 3 is the sampler; binding 4 is the uniform buffer. *Why:* the layout is fixed per export; per-frame bind groups are created from it.
- `pipeline: wgpu::RenderPipeline` - compiled from the embedded `shader.wgsl` (vertex `vs_main`, fragment `fs_main`; no depth, no blend, no MSAA). *Why:* compiled once at startup; compilation is the expensive step.
- `out_tex: wgpu::Texture` / `out_view: wgpu::TextureView` - the `out_w x out_h` render target in `FORMAT`; flagged `RENDER_ATTACHMENT | COPY_SRC`. *Why:* the render pass writes here; `copy_texture_to_buffer` reads it.
- `readback: wgpu::Buffer` - CPU-mappable buffer sized `padded_bpr * out_h` bytes; flagged `COPY_DST | MAP_READ`. *Why:* GPU-to-CPU pixel readback must go through a mappable buffer.
- `padded_bpr: u32` - bytes per row rounded up to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`. *Why:* the readback buffer has alignment padding per row that must be stripped before returning the pixel slice.

### Used by

- `src-tauri/src/export/gpu_compositor.rs` - `GpuCompositor` holds a `Gpu` and calls `upload_tex` each frame.

## Gpu::new

```rust
pub fn new(out_w: u32, out_h: u32) -> Option<Gpu>
```

Creates the full device-lifetime GPU state for an `out_w x out_h` output texture. Returns `None` if no adapter is found or device creation fails.

### Inputs

- `out_w: u32, out_h: u32` - output frame dimensions. *Why:* the output texture and readback buffer are sized to these at creation time.

### Returns

`Some(Gpu)` on success; `None` if no adapter or device creation fails. The `exporter` falls back to `CpuCompositor` on `None`.

### Implementation

1. Create a default wgpu `Instance`; request a default adapter (synchronous via `pollster`).
2. Raise texture size limits: start from `downlevel_defaults` and call `.using_resolution(adapter.limits())` so 4K screen textures (> 2048 px, the downlevel cap) are accepted.
3. Request a device with the raised limits.
4. Create a shared bilinear clamp-to-edge sampler.
5. Call `make_bind_layout` (private): 3 texture bindings + sampler + uniform buffer.
6. Call `make_pipeline` (private): load `shader.wgsl` from `include_str!`, create pipeline layout, build the render pipeline.
7. Allocate the output texture (`out_tex`) and its view.
8. Compute `padded_bpr = align_up(out_w * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)` and allocate the readback buffer.

## Gpu::upload_tex

```rust
pub fn upload_tex(&self, label: &str, data: &[u8], w: u32, h: u32) -> wgpu::Texture
```

Creates a new `w x h` `FORMAT` texture and immediately uploads `data` (BGRA, `w * h * 4` bytes) via `create_texture_with_data`. The texture is usable as `TEXTURE_BINDING | COPY_DST`.

### Inputs

- `label: &str` - debug label visible in GPU capture tools. *Why:* helps distinguish bg/screen/webcam uploads in frame captures.
- `data: &[u8]` - BGRA pixel bytes. *Why:* must match `FORMAT`; the format is `Bgra8Unorm` throughout.
- `w: u32, h: u32` - texture dimensions. *Why:* the texture is sized to the source exactly; the shader handles the UV mapping.

### Returns

A new `wgpu::Texture`. A fresh texture is created on every call; the caller owns it and drops it when the frame is done.
