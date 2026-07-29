# src-tauri/src/export/gpu/gpu_pipeline.rs

Bind-group-layout and render-pipeline construction for `Gpu::new`, split out of `gpu/mod.rs` so that file stays under the size limit. Pure builders - no behavior change of their own; both are called exactly once per process, when the shared device is first wrapped in a `Gpu`.

## make_bind_layout

```rust
pub(super) fn make_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout
```

Builds the fixed six-binding layout that every per-frame bind group (see `gpu_compositor_tex::build_resources`) is created against.

### Inputs

- `device: &wgpu::Device` - the shared device to create the layout on. *Why:* bind group layouts are device-scoped resources.

### Returns

A `wgpu::BindGroupLayout` with:
- bindings 0-3: `texture_2d<f32>`, fragment-visible, filterable float sample type, not multisampled - bg, screen Y, screen UV, and webcam respectively. *Why one shape for all four:* the Y (`R8Unorm`) and UV (`Rg8Unorm`) planes are narrower formats than bg/webcam (`Bgra8Unorm`), but all four are still sampled as `Float { filterable: true }`, so one closure (`tex(binding)`) builds all four entries. The binding-order convention (0=bg, 1=Y, 2=UV, 3=webcam) is shared with `gpu_compositor_tex::build_resources` and the WGSL shader's `@binding` attributes, not enforced by the type system - the three must be kept in sync by hand.
- binding 4: a filtering `Sampler`.
- binding 5: a `Uniform` buffer, no dynamic offset, no minimum size constraint.

### Used by

- `src-tauri/src/export/gpu/mod.rs` - `Gpu::new` calls this once and stores the result as `Gpu::bind_layout`.
- `src-tauri/src/export/gpu/gpu_compositor_tex.rs` - `build_resources` creates each frame-size's bind group against this layout.

## make_pipeline

```rust
pub(super) fn make_pipeline(device: &wgpu::Device, bind_layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline
```

Compiles the embedded `shader.wgsl` and builds the render pipeline used for every composite pass.

### Inputs

- `device: &wgpu::Device` - the shared device to compile the shader module and build the pipeline on.
- `bind_layout: &wgpu::BindGroupLayout` - the layout from `make_bind_layout`, wrapped in a pipeline layout with no push constant ranges.

### Returns

A `wgpu::RenderPipeline` with vertex entry `vs_main` and fragment entry `fs_main` (both from `shader.wgsl`), no vertex buffers (the full-screen triangle is generated from `@builtin(vertex_index)` in the shader, not from vertex data), one color target in `FORMAT` with `blend: None` (the shader resolves all panel/ring blending itself via `mix`, so the fixed-function blend stage stays off) and `ColorWrites::ALL`, no depth/stencil, no multisampling, no pipeline cache.

### Implementation

1. Load `shader.wgsl` via `include_str!` and create a shader module from it.
2. Build a pipeline layout from `bind_layout` with an empty `push_constant_ranges`.
3. Create the render pipeline: vertex stage `vs_main` with `buffers: &[]`; fragment stage `fs_main` targeting `FORMAT` with `blend: None` and `write_mask: ColorWrites::ALL`; default `PrimitiveState` and `MultisampleState`; `depth_stencil: None`, `cache: None`.

### Used by

- `src-tauri/src/export/gpu/mod.rs` - `Gpu::new` calls this once and stores the result as `Gpu::pipeline`.
