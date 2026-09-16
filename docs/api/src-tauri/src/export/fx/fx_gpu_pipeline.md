# src-tauri/src/export/fx/fx_gpu_pipeline.rs

The wgpu resources `GpuFx` is built from, split out of `fx_gpu.rs` so that file holds only the renderer. Nothing here keeps state: both functions are called once from `GpuFx::new` (and `r8_texture` again from `GpuFx::apply`, whenever the glass mask changes) and hand back finished GPU objects.

## build_pipeline

```rust
pub(super) fn build_pipeline(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline)
```

The FX bind-group layout and render pipeline. Four bindings, all fragment-stage: 0 the composited frame, 1 its sampler, 2 the `FxU` uniform buffer, 3 the glass mask (the 1x1 white `blank` texture when no lens is active, so the layout never changes shape). The WGSL module is `concat!(include_str!("fx.wgsl"), include_str!("fx_clicks.wgsl"), include_str!("fx_lens.wgsl"), include_str!("mask/fx_mask.wgsl"), include_str!("fx_grade.wgsl"))` compiled as ONE module - see `fx_gpu.md` for why that concatenation is safe - and the `include_str!` paths resolve beside this file, which is why the shader files live in this directory or, for `fx_mask.wgsl`, in the `mask/` folder its Rust half already occupies. Order in the concat does not matter: WGSL resolves functions and module constants across the whole module, so `fs_main` in `fx.wgsl` may call `mask_fx` and `grade_fx` from files concatenated after it, and a sibling may be appended anywhere as long as its names do not collide (`fx_grade.wgsl` prefixes its constants `G_` for that reason). What the siblings all depend on is `fx.wgsl` coming first, because it declares `struct FxU` and the `u` binding they read. One full-screen triangle (`vs_main` -> `fs_main`), no blending, `FORMAT` as the target.

## r8_texture

```rust
pub(super) fn r8_texture(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, a: &[u8]) -> wgpu::TextureView
```

Upload `a` as a single-channel `R8Unorm` texture and return its view. `queue.write_texture`, not a buffer copy, so an arbitrary sprite width needs no 256-byte row padding - the masks are content-cropped and almost never a multiple of 256.
