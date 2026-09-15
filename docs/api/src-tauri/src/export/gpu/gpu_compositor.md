# src-tauri/src/export/gpu/gpu_compositor.rs

GPU-accelerated compositor backed by wgpu. Implements the `Compositor` trait by uploading the nv12 screen (as separate Y/UV textures) and the BGRA webcam into a lazily-(re)built resource bundle, running the WGSL compositing shader (which converts nv12 -> RGB via `screen_rgb`), and reading back BGRA pixels to a `Vec<u8>` that matches the `CpuCompositor` output format exactly (verified by the `cpu_gpu_parity_two_panels` test). It also builds that bundle (`build_resources`) and decides when the background texture is stale (`bg_key`, `should_upload`). Its tests live in the sibling `gpu_compositor_tests.rs` (`#[path]`-included); each no-ops when the machine has no usable wgpu adapter.

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

Everything that depends on frame size, built in one shot by `build_resources`. `sw/sh/ww/wh` record the sizes this bundle was built for, so the next `composite_into` call can detect a size change and trigger a rebuild; `bg_key` records the CONTENT key (`bg_key`) of the background actually sitting in `bg_tex`, `None` on every (re)build. It replaced a plain `bg_uploaded: bool`, which only ever flipped `false` on a rebuild - and since `FrameRenderer::reload_edit` rebuilds `bg` at the SAME dimensions, no dimension-driven rebuild ever fires for a background edit, so every newly-built background buffer was handed to `composite_into` and thrown away while the shader kept sampling the stale texture.

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
4. If `rebuild`, call `build_resources(g, sw, sh, ww, wh, ow, oh, &u)` and replace the cached bundle - this allocates fresh Y/UV/webcam/bg textures, a new uniform buffer, and a new bind group, and resets `bg_key` to `None`.
5. Upload the nv12 screen into the (possibly just-rebuilt) bundle: `update_tex_bpp` the Y plane (`screen[..sw*sh]`, `R8`, 1 byte/px) and the interleaved UV plane (`screen[sw*sh..]`, half-res, `Rg8`, 2 bytes/px); `update_tex` the webcam (BGRA, 4 bytes/px).
6. Compute `bg_key(bg)` - unless the background is DYNAMIC (a video/GIF asset; see `Compositor::set_bg_dynamic`), in which case the key is `None` and never computed, because that path re-uploads either way and the hash would be pure cost - and upload the background when `should_upload(r.bg_key != key, dynamic)` says so, then store it - so a rebuild re-uploads into the fresh `bg_tex`, AND a same-size background EDIT re-uploads too. The key is a length-seeded FNV-1a over ~4k strided samples: ~0.1-0.3 ms per frame against an ~8 MB buffer (cache-miss bound, not arithmetic), which is why it is computed BEFORE the `res` lock is taken - the work needs nothing from the cached bundle and must not widen the critical section.
7. `write_buffer` this frame's `Uniforms` into `ubuf` unconditionally (the buffer itself is only recreated on rebuild; its contents refresh every frame).
8. Begin a command encoder; start a render pass that clears to black and draws 3 vertices (full-screen triangle - no vertex buffer needed).
9. `copy_texture_to_buffer` from `out_tex` to `readback` using `padded_bpr`.
10. Submit the encoder; map the readback buffer in read mode; poll with `Maintain::Wait` for synchronous completion.
11. Clear `out`. When `padded_bpr` equals `out_w * 4` (no row padding - true whenever the output width is already a multiple of the 256-byte alignment), one contiguous `extend_from_slice`; otherwise resize `out` and copy row-by-row, stripping the padding. Unmap.

### Behaviors worth knowing

- `screen_panel_composites_onto_background` (unit test): same geometry as the CPU test - a 4x4 red screen (built via `bgra_to_nv12`, uploaded as Y/UV textures, converted back by `screen_rgb`) at (2,2) in an 8x8 blue background must leave the corner blue and the panel interior approximately red - within a few LSBs, not bit-exact.
- `cpu_gpu_parity_cover_crops_a_wide_webcam_into_a_square_panel` (unit test): a 3:1 webcam composited into a square panel shows the middle third on BOTH paths - `compositor::cover_rect` and the shader's `cover_uv` must agree, since one decode box now serves every layout.
- `cpu_gpu_parity_two_panels` (unit test): at three sampled points (bg corner, screen panel center, camera panel center) CPU and GPU outputs must agree within 3 quantization steps. Loosened from 2: the screen panel now round-trips through two independent nv12->RGB converts (CPU's u8-rounded `nv12_to_bgra` vs. the shader's float `screen_rgb`), which can differ by one extra LSB.

## build_resources

```rust
fn build_resources(
    g: &Gpu, sw: u32, sh: u32, ww: u32, wh: u32, ow: u32, oh: u32, u: &Uniforms,
) -> CompositorResources
```

Builds every texture, the uniform buffer, and the bind group for one `(sw, sh, ww, wh)` size combination - everything `GpuCompositor::composite_into` needs to render a frame at that screen/webcam size. Called only when `composite_into` detects a size change from its cached `CompositorResources` (or on the very first frame, when there is no cached bundle yet).

### Inputs

- `g: &Gpu` - the shared device/queue/pipeline/sampler/bind-group-layout. *Why:* every texture, buffer, and the bind group are created against `g.device`; the pipeline's bind-group layout (`g.bind_layout`, from `gpu::make_bind_layout`) fixes the six binding slots used below.
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

- `GpuCompositor::composite_into` in this file - calls this exactly when `(sw, sh, ww, wh)` differs from the cached `CompositorResources` (or there is no cached bundle yet).

## bg_key

```rust
fn bg_key(bg: &[u8]) -> u64
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
fn should_upload(key_changed: bool, dynamic: bool) -> bool
```

Does this frame's background need uploading to the GPU? A STATIC background uploads only when its content key moved - that is the whole point of `bg_key`, and it saves an ~8 MB texture write on every frame of a normal export. A DYNAMIC one (a video/GIF asset - `Compositor::set_bg_dynamic`, decided by `scene::background::video_source`) uploads unconditionally, for the reason above. The caller skips computing the key at all in that case, so the dynamic path is also the cheaper one per frame.

### Behaviors worth knowing

- `a_dynamic_background_always_uploads_a_static_one_only_on_change` (unit test): all four combinations, including the one that matters - `should_upload(false, true)` is `true`.
