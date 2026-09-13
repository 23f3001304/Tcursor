# src-tauri/src/export/gpu/gpu_compositor_tex.rs

Texture and bind-group construction for `GpuCompositor::composite_into`, split out of `gpu_compositor.rs` so that file stays under the size limit, plus `bg_key` (the background's change key, which lives here because it is about the freshness of a resource this file builds).

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

A `CompositorResources` with `bg_key: None`. *Why:* the caller (`composite_into`) is responsible for uploading the background into the freshly-created `bg_tex` on the same call that triggered the rebuild; returning the bundle with no key recorded is what makes that upload happen on the very next comparison.

### Implementation

1. Create `screen_y_tex` (`R8Unorm`, `sw x sh`) and `screen_uv_tex` (`Rg8Unorm`, `(sw/2).max(1) x (sh/2).max(1)`) via `Gpu::create_tex_fmt`.
2. Create `webcam_tex` and `bg_tex` (both `FORMAT`, i.e. BGRA) via `Gpu::create_tex`.
3. Create the uniform buffer `ubuf` via `wgpu::util::DeviceExt::create_buffer_init`, seeded with `bytemuck::bytes_of(u)`, usage `UNIFORM | COPY_DST`.
4. Create a texture view for each of the four textures (Y, UV, bg, webcam).
5. Build the bind group against `g.bind_layout`: binding 0 = bg view, 1 = screen Y view, 2 = screen UV view, 3 = webcam view, 4 = `g.sampler`, 5 = `ubuf.as_entire_binding()`.
6. Return `CompositorResources { sw, sh, ww, wh, screen_y_tex, screen_uv_tex, webcam_tex, bg_tex, ubuf, bind, bg_key: None }`.

### Used by

- `src-tauri/src/export/gpu/gpu_compositor.rs` - `GpuCompositor::composite_into` calls this exactly when `(sw, sh, ww, wh)` differs from the cached `CompositorResources` (or there is no cached bundle yet).

## bg_key

```rust
pub(super) fn bg_key(bg: &[u8]) -> u64
```

A cheap CONTENT key for the background buffer: FNV-1a seeded with `bg.len()` and folded over ~4096 evenly strided 4-byte samples. `GpuCompositor::composite_into` re-uploads `bg_tex` exactly when this key changes.

### Why a hash and not a dimension check or a full compare

`FrameRenderer::reload_edit` rebuilds `bg` in place at the SAME `out_w x out_h` whenever the background settings change, so the resource-rebuild condition (`sw/sh/ww/wh`) never fires for a background edit - the old `bg_uploaded` flag stayed `true`, every rebuilt buffer was discarded, and the warm preview kept rendering the previous background until something forced a full `build_renderer` (a folder or aspect change). A full hash (or memcmp) of an ~8 MB BGRA buffer every frame would cost more than the upload it saves; a background that changes at all - fill colour, gradient, blur radius, wallpaper - changes across the whole frame, so a strided sample sees it.

### Where the sampling is NOT enough

The sample is why a VIDEO background cannot be decided by this key at all: a frame that moves only a small region can hash equal to the frame before it, and "probably changed" is the difference between a moving background and one frozen on screen. `should_upload` below is the exception that covers it.

### Behaviors worth knowing

- `bg_key_tracks_content_not_just_length` (unit test): identical buffers key identically; a one-channel change across every pixel changes the key; a different length changes the key; an empty slice is stable and does not panic.

## should_upload

```rust
pub(super) fn should_upload(key_changed: bool, dynamic: bool) -> bool
```

Does this frame's background need uploading to the GPU? A STATIC background uploads only when its content key moved - that is the whole point of `bg_key`, and it saves an ~8 MB texture write on every frame of a normal export. A DYNAMIC one (a video/GIF asset - `Compositor::set_bg_dynamic`, decided by `scene::background::video_source`) uploads unconditionally, for the reason above. The caller skips computing the key at all in that case, so the dynamic path is also the cheaper one per frame.

### Behaviors worth knowing

- `a_dynamic_background_always_uploads_a_static_one_only_on_change` (unit test): all four combinations, including the one that matters - `should_upload(false, true)` is `true`.
