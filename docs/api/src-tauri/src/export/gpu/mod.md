# src-tauri/src/export/gpu/mod.rs

wgpu device/pipeline initialization, GPU availability probe, texture upload helper, and the shared `FORMAT` constant for the export GPU compositor. The expensive wgpu device is created ONCE per process (`shared_device`) and shared behind `Arc` by every `Gpu`/`GpuFx`; `Gpu::new` allocates only the dims-dependent pipeline + output texture + readback buffer on top of it. Per-frame paths touch only `upload_tex` and the already-allocated device/queue.

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

- `src-tauri/src/export/gpu/mod.rs` - `Gpu::new` uses it to compute `padded_bpr`.

## shared_device

```rust
pub fn shared_device() -> Option<(Arc<wgpu::Device>, Arc<wgpu::Queue>)>
```

The process-global device + queue as `Arc` handles all pointing at the SAME underlying wgpu device (wgpu's own `Device`/`Queue` are not `Clone`, so they are shared behind `Arc`). Created lazily on first call and cached for the process lifetime (a `None` result is cached too, so a machine with no adapter settles onto the CPU compositor once instead of retrying every frame). Blocks the calling thread via `pollster::block_on` on first init only.

### Returns

`Some((device, queue))` sharing the one process device; `None` if no usable adapter. Because `Arc<Device>` derefs to `Device`, callers store these in `Gpu`/`GpuFx` and use `.device`/`.queue` exactly as before.

### Why

`request_device` costs tens-to-hundreds of ms (driver init) and nothing about the device depends on output dims or aspect. Sharing it means an aspect change, a new export, and the first preview no longer each pay that cost - and `build_renderer` no longer spins up TWO devices (compositor + fx) every time the frame resizes. This is the fix for the aspect-change preview lag.

### Used by

- `src-tauri/src/export/gpu/mod.rs` - `Gpu::new` builds its pipeline/textures on the shared device.
- `src-tauri/src/export/fx/fx_gpu.rs` - `GpuFx::new` shares the same device instead of creating a second one.

## gpu_available

```rust
pub fn gpu_available() -> bool
```

Returns `true` if a usable GPU device could be created, warming the `shared_device` cache as a side effect. Blocks the calling thread via `pollster::block_on` on first init.

### Returns

`true` if the shared device initialized; `false` if this machine has no usable adapter.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - gates GPU vs CPU FX rendering on it before building a `GpuFx` (`select_compositor` in `exporter.rs` does NOT call this - it just tries `GpuCompositor::new` and falls back to CPU on `None`).

## FORMAT

```rust
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
```

The texture format used for all export textures (background, screen, webcam, output render target). `Bgra8Unorm` keeps channel order matching `CpuCompositor`'s BGRA layout so readback bytes require no channel swap and both paths produce identical pixel data.

### Used by

- `src-tauri/src/export/gpu/mod.rs` - `Gpu::new` and `Gpu::upload_tex` set this format on every texture.
- `src-tauri/src/export/gpu/gpu_compositor.rs` - implicitly via the bind group and render pass.

## Gpu

```rust
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
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

- `device: Arc<wgpu::Device>` - the shared, process-global wgpu logical device (see `shared_device`). *Why:* all resource creation (buffers, textures, bind groups) goes through this; `Arc` so every `Gpu`/`GpuFx` reuses the one device.
- `queue: Arc<wgpu::Queue>` - the command submission queue of the shared device. *Why:* `submit` and texture uploads are dispatched here.
- `sampler: wgpu::Sampler` - bilinear/linear filter, clamp-to-edge on all axes. *Why:* shared sampler avoids creating one per frame; all texture lookups in the shader use the same filter settings.
- `bind_layout: wgpu::BindGroupLayout` - five entries: bindings 0-2 are 2D float-filterable textures (bg, screen, webcam); binding 3 is the sampler; binding 4 is the uniform buffer. *Why:* the layout is fixed per export; per-frame bind groups are created from it.
- `pipeline: wgpu::RenderPipeline` - compiled from the embedded `shader.wgsl` (vertex `vs_main`, fragment `fs_main`; no depth, no blend, no MSAA). *Why:* compiled once at startup; compilation is the expensive step.
- `out_tex: wgpu::Texture` / `out_view: wgpu::TextureView` - the `out_w x out_h` render target in `FORMAT`; flagged `RENDER_ATTACHMENT | COPY_SRC`. *Why:* the render pass writes here; `copy_texture_to_buffer` reads it.
- `readback: wgpu::Buffer` - CPU-mappable buffer sized `padded_bpr * out_h` bytes; flagged `COPY_DST | MAP_READ`. *Why:* GPU-to-CPU pixel readback must go through a mappable buffer.
- `padded_bpr: u32` - bytes per row rounded up to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`. *Why:* the readback buffer has alignment padding per row that must be stripped before returning the pixel slice.

### Used by

- `src-tauri/src/export/gpu/gpu_compositor.rs` - `GpuCompositor` holds a `Gpu` and calls `upload_tex` each frame.

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

1. Get the shared process-global device + queue via `shared_device` (created once, reused across every compositor/fx build and export; `None` if no adapter). The adapter enumeration + `request_device` cost is paid only on the very first call, process-wide.
2. Create a bilinear clamp-to-edge sampler.
3. Call `make_bind_layout` (private): 3 texture bindings + sampler + uniform buffer.
4. Call `make_pipeline` (private): load `shader.wgsl` from `include_str!`, create pipeline layout, build the render pipeline.
5. Allocate the output texture (`out_tex`) and its view - sized to `out_w x out_h`, the one per-instance dims-dependent part.
6. Compute `padded_bpr = align_up(out_w * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)` and allocate the readback buffer.

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

## gpu_compositor

GPU-accelerated compositor implementing the `Compositor` trait via wgpu + a WGSL compositing shader. Key items: `GpuCompositor` (holds `Gpu` plus a `OnceLock` background texture), `GpuCompositor::new(out_w, out_h) -> Option<GpuCompositor>`, `GpuCompositor::composite_into`.

## gpu_uniforms

Defines the GPU uniform buffer layout and the builder that populates it from per-frame scene, camera, and layout state. Key items: `Uniforms` (`repr(C)` bytemuck-castable struct with UV panel bounds, zoom center, radii, alphas, sizes), `build_uniforms(scene, cam, layout, has_webcam) -> Uniforms`.

## compositor

Defines the `Compositor` trait shared by CPU and GPU implementations, and provides the software `CpuCompositor`. Key items: `Compositor` trait with `composite_into(screen, sw, sh, webcam, cam, bg, layout, scene, out: &mut Vec<u8>)`; `CpuCompositor` (zero-state, always available).
