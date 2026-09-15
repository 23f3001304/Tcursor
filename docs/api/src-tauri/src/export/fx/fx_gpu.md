# src-tauri/src/export/fx/fx_gpu.rs

GPU-backed FX renderer that uploads a composited BGRA frame to wgpu, runs the FX shader (spotlight, click effects, video FX) in a full-screen triangle pass, and reads the result back into the caller's buffer.

**The shader is TWO files compiled as one module.** `fx_gpu_pipeline::build_pipeline` (its own file since the split) builds the WGSL source as `concat!(include_str!("fx.wgsl"), <newline>, include_str!("fx_clicks.wgsl"))`: `fx.wgsl` holds the uniform struct `FxU`, the bindings, the `FX_*`/`SP_*`/`VF_*` ids, the vertex stage, the noise/nebula helpers, the shockwave UV warp + chromatic dispersion (which must happen before the texture sample) and the spotlight/video-FX blending; `fx_clicks.wgsl` holds the shared click timing (`fx_ease`, `fx_alpha`), the small coverage helpers and one function, `clicks(base, px, oh, style, n) -> vec3<f32>`, which `fs_main` calls last. *Why split:* the click styles are the half that changes, and they are readable on their own once the frame-wide effects are not interleaved with them. *Why this order is safe:* WGSL resolves module-scope declarations out of order, so `fs_main` calling `clicks()` (and the shockwave block calling `fx_ease`) from the half appended after it is legal; the GPU tests below compile the real module, so a violation fails the suite rather than shipping.

Selected automatically by `select_fx` when a wgpu adapter is available; falls through to `CpuFx` otherwise - and falls back to it per frame if a readback ever fails (see `GpuFx::apply`). Its tests live in the sibling `fx_gpu_tests.rs`, the house convention for every module here.

## GpuFx

```rust
pub struct GpuFx {
    device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout, pipeline: wgpu::RenderPipeline,
    out_tex: wgpu::Texture, out_view: wgpu::TextureView, readback: wgpu::Buffer, padded_bpr: u32,
}
```

Holds all persistent wgpu objects for one export session. Constructed once in `GpuFx::new` and reused across every frame to avoid per-frame overhead. The device/queue are the process-global shared handles (`gpu::shared_device`), so `GpuFx` no longer stands up its own wgpu device. All fields are private; the struct is opaque to callers.

### Fields

- `device` / `queue` - *the shared process-global logical device and its submission queue (`Arc`, see `gpu::shared_device`); all GPU work is submitted through these.*
- `sampler` - *bilinear (linear mag + min) sampler bound at binding 1 so the composited frame is sampled smoothly by the full-screen triangle.*
- `bind_layout` - *bind group layout declaring bindings 0 (texture), 1 (sampler), 2 (uniform); cached so per-frame bind groups can be created cheaply.*
- `pipeline` - *render pipeline compiled once at construction from the concatenated `fx.wgsl` + `fx_clicks.wgsl` source (both embedded via `include_str!`); reused every frame.*
- `out_tex` / `out_view` - *fixed-size `RENDER_ATTACHMENT | COPY_SRC` output texture; the shader renders into it each frame, then its contents are copied to `readback`.*
- `readback` - *`COPY_DST | MAP_READ` buffer sized `padded_bpr * oh`; mapped after each submit so the CPU can read back processed pixels.*
- `padded_bpr` - *bytes per row rounded up to `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`; used to strip padding when copying from `readback` back into the caller's buffer.*

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `select_fx` constructs a `GpuFx` and boxes it as a `dyn FxRenderer`.

### GpuFx::mask

```rust
mask: Mutex<Option<(u64, wgpu::TextureView)>>,
blank: wgpu::TextureView,
```

The cursor lens mask (`lens::LensMask`), uploaded once per (pack, kind) and kept until a different one is asked for, plus the 1x1 opaque texture bound when no lens is live (the bind group layout always needs a binding at 3).

*Why one entry is enough:* a frame draws ONE cursor. A settled cursor re-uses the upload forever; a state change re-uploads about 16 KB per frame for the ten frames the morph lasts, because `morph_mask`'s key carries the step.

*Why a `Mutex`:* `FxRenderer::apply` takes `&self`. A poisoned lock is recovered rather than propagated, for the same reason `readback_into` never panics - this runs on a Tauri command thread for the editor preview, where a panic takes out the overlay rather than one frame of it.

### GpuFx::blank

See `GpuFx::mask`.

## GpuFx::new

```rust
pub fn new(ow: u32, oh: u32) -> Option<GpuFx>
```

Initializes the full wgpu stack for a frame of size `ow x oh`. Returns `None` if no adapter is available, enabling a clean fallback to the CPU path without panicking.

### Inputs

- `ow: u32` - output frame width in pixels. *Why:* determines the width of `out_tex`, the `readback` buffer length, and `padded_bpr`.*
- `oh: u32` - output frame height in pixels. *Why:* determines the height of `out_tex` and the `readback` buffer length.*

### Returns

`Option<GpuFx>` - `Some` with a fully initialized renderer when a compatible adapter is found; `None` when `gpu::shared_device` fails (headless CI, remote server, absent GPU driver).

### Implementation

1. Get the shared process-global device + queue via `gpu::shared_device` (blocking `pollster` init on the first call process-wide; `None` if the system has no usable GPU). This shares the compositor's device rather than creating a second one - removing half the device-creation cost behind an aspect change / frame resize.
2. Create a bilinear `Sampler`. *Why bilinear:* the full-screen triangle samples the composited frame at exact pixel centers, so filtering is safe and prevents aliasing at spotlight feather edges.*
3. Call `build_pipeline` to compile the concatenated `fx.wgsl` + `fx_clicks.wgsl` source and produce the bind layout and render pipeline.
4. Allocate `out_tex` as `RENDER_ATTACHMENT | COPY_SRC` at `ow x oh`. The shader renders into this texture; it is then copied to the readback buffer.
5. Compute `padded_bpr = align_up(ow * 4, COPY_BYTES_PER_ROW_ALIGNMENT)` and allocate `readback` as `COPY_DST | MAP_READ` with total size `padded_bpr * oh`.

## GpuFx::apply

```rust
fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState)
```

Implements `FxRenderer::apply`. Uploads `out` as the composited source texture, dispatches the FX shader, and writes the processed result back into `out` in-place.

### Inputs

- `out: &mut [u8]` - composited BGRA frame (`ow * oh * 4` bytes); modified in-place with the GPU-rendered result. *Why in-place:* avoids an extra allocation; the caller owns this buffer and passes it directly to the encoder after this call.*
- `ow: u32`, `oh: u32` - frame dimensions. *Why:* used for texture upload dimensions and readback row-stride calculations.*
- `state: &FxState` - renderer-agnostic FX description (spotlight, click hits, video FX). *Why:* decouples the GPU renderer from effect parameterization; `build_fx_u` translates this to the shader uniform.*

### Uniforms/fields it reads, and why

- `state.spot` - spotlight center, dim factor, radius/feather fractions, alpha, mode, tint. *Why:* packed into `FxU.b/c/d/tint` by `build_fx_u` so `fx.wgsl` can apply the spotlight blend.*
- `state.hits` - up to `MAX_HITS` click positions and progress values. *Why:* each hit drives an animated click effect (ripple, pulse, etc.) branch in the fragment shader.*
- `state.style`, `state.color`, `state.intensity` - click effect variant and color parameters. *Why:* the shader selects a click branch by `style_id` and blends by `color` / `intensity`.*
- `state.video` - optional full-frame video FX mode and alpha. *Why:* enables nebula wash, cinematic dim, and other full-frame effects in the shader.*

### Returns

`()` - processed pixels are written back into `out`; all return state is implicit.

### Failure handling

`apply` **never panics on a readback failure.** The map+copy step lives in `readback_into`, which reports `false` instead of unwrapping; `apply` then warns once (a `std::sync::Once`, so a repeating device fault cannot spam per frame) and re-renders that one frame with `CpuFx`.

*Why a CPU fallback is correct rather than a visible glitch:* readback fails before a single byte is written, so `out` still holds exactly the composited frame that was uploaded as the shader's source - the CPU path can simply redo the effect on it. The two renderers agree on everything except the documented approximations (see `spotdraw.md`), pinned by `cpu_spotlight_dim_matches_the_shader_at_probe_points` and `cpu_click_ring_gains_match_the_shader`, so a fallback frame is at worst slightly different in Blur/Nebula/Particles, not wrong.

*Why this matters beyond one frame:* `apply` used to `.expect("map fx readback")` inside the `map_async` callback. That was survivable on the export thread, but the editor preview reaches this code from a Tauri command thread via `preview_fx::with_fx`, where an unwind would have poisoned the overlay's renderer cache and disabled the FX preview for the whole session.

*Why it is not unit-tested:* forcing a wgpu map failure needs either a device-loss injection point or a mock `wgpu::Buffer`, neither of which the current seam offers cheaply. The reasoning it rests on is checked instead: `out` is untouched on the failure branch (the copy loop is the only writer, and it is inside the success path), and `CpuFx` produces a comparable frame from the same input (the two parity tests above).

## GpuFx::readback_into

```rust
fn readback_into(&self, out: &mut [u8], ow: u32, oh: u32) -> bool
```

Maps the readback buffer, copies the rendered frame into `out` row by row (dropping the `COPY_BYTES_PER_ROW_ALIGNMENT` padding), and unmaps. Returns `false` if the map failed, leaving `out` untouched.

The map result travels back over an `mpsc::channel` from the `map_async` callback rather than being unwrapped inside it. `device.poll(Maintain::Wait)` guarantees the callback has run by the time it returns, so a `try_recv` that does not yield `Ok(true)` means a genuine failure, not a race.

### Behaviors

These tests skip on a machine with no wgpu adapter (`GpuFx::new` returns `None`) - and say so on stderr, so `cargo test fx -- --nocapture` distinguishes "green because it passed" from "green because it never ran". Where an adapter exists they are the CPU-to-GPU parity harness, since the shader is the reference look for `fxdraw.rs`.

- `spotlight_dims_corner_more_than_center` - the shader's own smoke test: a `Classic` spotlight leaves the centre brighter than the corner.
- `cpu_spotlight_dim_matches_the_shader_at_probe_points` - `CpuFx` and `GpuFx` render the same `Classic` spotlight within ±3/255 at five probe points spanning the lit core, the feather band, and the fully dimmed corners. Pins the feather curve and the dim compositing formula against drift in either direction.
- `cpu_click_ring_gains_match_the_shader` - for Neon, Shockwave, Ripple, Glow and Pulse, at progress 0.05, 0.5 and 0.8, the *total light added over the base frame* agrees between the two paths within 20%. *Why total added light rather than per-pixel equality:* ring geometry is identical but sub-pixel antialiasing is not, so a sum is tight enough to catch a wrong additive gain (it was the guard that caught Shockwave at `0.7` where the shader uses `0.5`) without being brittle about edge pixels. *Why three progress points:* one point pins the gains, three pin the shared easing too - a `clickdraw.rs` that forgot `ease_out` would still match at a single sample but not across the curve. *Why Particles is excluded:* the GPU's `sin` approximation moves individual sparks (see `clickdraw.md`).
- `the_impact_flash_is_brightest_at_the_click_and_gone_by_p_0_2` - the shared white impact flash, probed on the shader itself at the hit pixel with a black tint on Particles (a style that only ever ADDS the tint, so a black one contributes nothing and the flash is the only thing moving): blown out at p=0, already fading at p=0.10, and back to the untouched base frame by p=0.20.

### Implementation

1. Upload `out` as a fresh `TEXTURE_BINDING | COPY_DST` texture (`fxframe`) via `create_texture_with_data`. *Why per-frame:* the composited frame changes every frame; reusing a persistent upload texture would require staging and mapping overhead with no net benefit.*
2. Call `build_fx_u(state, ow, oh)` to pack `FxState` into an `FxU`, then upload it as a `UNIFORM` buffer.
3. Build a fresh `BindGroup` with the frame texture view at binding 0, sampler at 1, uniform buffer at 2.
4. Begin a command encoder; open a render pass clearing `out_view` to black, set the pipeline and bind group, draw 3 vertices (full-screen triangle - no vertex buffer required).
5. Encode `copy_texture_to_buffer` from `out_tex` into `readback` using the padded layout.
6. Submit the encoder and call `device.poll(Maintain::Wait)` to synchronize. *Why synchronous:* the composite stage must have the result in `out` before handing the frame to the encoder.*
7. Map `readback` for reading. For each of the `oh` rows, copy `ow * 4` bytes, skipping the `padded_bpr - ow*4` padding bytes, into `out`. *Why strip padding:* wgpu row-alignment padding must not appear in the raw frame written to the encoder.*
8. Unmap `readback`.

### Behaviors worth knowing

- `spotlight_dims_corner_more_than_center` - with a centered spotlight at `(32, 32)` on a 64x64 frame, the corner pixel is darker than the center pixel, confirming the GPU path is active and the readback is correctly unpadded. Test is skipped with `return` when no adapter is available.
