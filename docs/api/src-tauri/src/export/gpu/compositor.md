# src-tauri/src/export/gpu/compositor.rs

Defines the `Compositor` trait shared by the CPU and GPU implementations, and provides the software `CpuCompositor` that blends the background, screen, and webcam panels using `fast_image_resize` and a per-pixel rounded-box SDF. CPU only - the GPU path lives in `gpu_compositor.rs`.

## Compositor

```rust
pub trait Compositor: Send + Sync {
    fn composite_into(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
        out: &mut Vec<u8>,
    );

    fn set_bg_dynamic(&self, _dynamic: bool) {}
}
```

Common interface for software and hardware compositors.

### Inputs

- `screen: &[u8]` - the captured screen frame in **nv12** (`sw * sh * 3/2` bytes: a full-res Y plane followed by an interleaved half-res UV plane), not BGRA. *Why nv12:* ~2.6x smaller than BGRA, so far fewer bytes cross the ffmpeg -> exporter pipe (the measured export bottleneck; see `color.rs`). `GpuCompositor` uploads Y/UV as separate textures and converts to RGB in the shader (`screen_rgb` in `gpu/shader.wgsl`); `CpuCompositor` instead converts up front via `color::nv12_to_bgra` so the rest of its blend path stays unchanged BGRA math.
- `sw: u32, sh: u32` - capture source dimensions. *Why:* needed to set up the resize crop.
- `webcam: Option<(&[u8], u32, u32)>` - optional `(bytes, width, height)` for the webcam frame. *Why:* `None` when there is no webcam, or after EOF; the compositor skips the camera panel in that case.
- `cam: Camera` - the virtual camera center and scale for whole-scene zoom. *Why:* drives the crop rectangle that simulates zoom on the base image.
- `bg: &[u8]` - BGRA background image, `out_w * out_h * 4` bytes. *Why:* used as the bottom layer; the screen panel is composited on top of it.
- `layout: &Layout` - output dimensions and padding. *Why:* determines the output buffer size and the inset geometry.
- `scene: &Scene` - the two panels (screen + camera) with rects, radii, and alphas. *Why:* drives placement, clipping, and blend weight for each panel.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller reuse (e.g. pool) the same allocation across frames instead of the compositor allocating fresh each call.

### Returns

Nothing (`()`). The impl resizes `out` to `out_w * out_h * 4` bytes and fully overwrites it with BGRA pixels.

### set_bg_dynamic

Tells the compositor whether `bg` now CHANGES every frame - that is, whether the background is a video/GIF asset (`export::scene::background::video_source` is the single answer to that question). Asserted by `FrameRenderer::new` and re-asserted by `reload_edit`, since an edit can flip a still background into a moving one and back.

*Why it exists:* `GpuCompositor` caches `bg` in a texture and re-uploads only when a sampled content key changes (`gpu_compositor_tex::bg_key`, ~4096 strided pixels). A video frame that moves only a small region can hash equal to its predecessor, which would freeze the background on a stale upload. With this set, that path uploads unconditionally and skips computing the key at all.

*Why a defaulted no-op rather than a required method:* `CpuCompositor` copies `bg` into its base buffer every frame regardless, so it has nothing to do - and a default keeps every existing implementation and test call site untouched.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::composite_at` calls `self.compositor.composite_into(...)` once per output frame; `new` and `render/bg.rs`'s `reload_edit` call `set_bg_dynamic`.
- `src-tauri/src/export/gpu/gpu_compositor.rs` - `GpuCompositor` implements this trait.

## CpuCompositor

```rust
pub struct CpuCompositor;
```

Software compositor with no GPU dependency; zero interior state, trivially `Send + Sync`.

### Implementation of `composite_into`

1. Convert the incoming nv12 `screen` to BGRA once via `color::nv12_to_bgra`, rebinding the local `screen` to the converted buffer - every step below (including the fast-path check) then runs on packed BGRA exactly as it did before nv12 was introduced.
2. Fast-path: when the scene is unzoomed (`cam.scale <= 1.0001`), the camera panel is invisible, the screen panel is fully opaque with no corner rounding or ring, and the converted `screen` is already exactly `out_w x out_h` BGRA, clear `out`, copy `screen` straight into it, and return - skipping steps 3-5 entirely for the common 1:1 no-PiP case.
3. Otherwise, copy `bg` into a working buffer `base` (`out_w * out_h * 4` bytes).
4. Draw the screen panel onto `base` via `draw_panel` with `cover = false`: resize the WHOLE `screen` to `panel.rect` dimensions and alpha-blend with rounded-rect SDF coverage. *Why no cover-crop here:* the screen panel's rect is already built from the source's own aspect (`inset_rect`, `Presenter`'s fitted rect, the `Camera` layout's `small_screen`), so a cover-crop would be a no-op that only risks rounding drift - and the screen must never be cropped, it aspect-fits.
5. Compute the zoom crop via `coordmap::crop(cam, ow, oh)` and resize `base` (crop -> full output) into `resized`, simulating the camera zoom. This zooms the entire base including the screen panel but not what will be drawn on top.
6. Clear `out` and copy `resized` into it.
7. Draw the camera panel on top of `out` via `draw_panel` with `cover = true` (see `cover_rect` below); the camera is not subject to zoom.

### cover_rect

```rust
fn cover_rect(sw: u32, sh: u32, pw: u32, ph: u32) -> (f64, f64, f64, f64)
```

The largest CENTRED sub-rect of a `sw x sh` source that has the `pw x ph` panel's aspect, as `(x, y, w, h)` in source pixels - a cover-fit crop: the mismatched axis is cropped, never squashed. Passed straight to `resize_crop`'s crop arguments by `draw_panel(.., cover = true)`.

**Why it exists:** the webcam is decoded ONCE per export into a single box (`render::meta::webcam_box`, at the source video's aspect) but the layout track can show it in panels of different aspects over time - a 16:9 `CamAspect::Wide` bubble in a `Screen` segment, a square big-cam in a `Camera`/`CameraOnly`/`Presenter` segment. Stretching one fixed box across whatever rect the layout produced distorted the face by 1.78x for whole segments, in one direction or the other depending on which panel the decode box had been shaped for. Cropping per panel at composite time makes every layout correct from the same decode, and matches the framing the editor's live JS preview already uses (`previewCanvas.ts`'s `coverDraw`). The GPU path does the identical crop in UV space (`shader.wgsl`'s `cover_uv`).

### draw_panel / rrect_sd_px / blit / blit_ring (Task 9 Part B - software ring mirror)

`draw_panel(dst, dw, dh, src, sw, sh, panel, cover)` (private) resizes `src` (all of it, or its `cover_rect` centre crop when `cover`) into `panel.rect` and alpha-blends it with rounded-rect SDF coverage (same formula as the GPU shader's `rrect_sd`, factored out here as the private `rrect_sd_px(tx, ty, pw, ph, r) -> f32` helper so both the panel-coverage closure and the ring blend share one SDF implementation). When `panel.ring_px > 0.0`, it then calls the private `blit_ring` to stroke a colored band just inside the panel edge, mirroring the WGSL shader's post-camera-mix ring blend pixel-for-pixel: `band = clamp((ring_px + d) / max(ring_px, 1.0), 0, 1)` for pixels with `-ring_px <= d <= 0`, weighted by the panel's own alpha, blended into the BGRA destination bytes (ring_color is RGB; `dst[0]=B, dst[1]=G, dst[2]=R`).

**`blit`/`blit_ring`'s origin (`ox`/`oy`) is SIGNED (`i32`), not `u32`.** `draw_panel` computes it as `panel.rect.x.round() as i32` (previously `.max(0.0).round() as u32`) - a panel that sits partly off the top/left edge (an off-canvas keyframed PiP, or any layout that places a panel outside the frame) must CLIP to its visible sub-rect, not snap its origin to 0 and jump the whole panel to the corner. Both functions map each panel-local pixel `(tx, ty)` to destination `(ox + tx, oy + ty)` and skip it when that lands outside `[0, dst_w) x [0, dst_h)` on EITHER edge - the near (negative) edge exactly like the far edge (`>= dst_w`/`>= dst_h`) already did. This naturally reads the correct VISIBLE sub-region of the resized/panel-local content (no separate source-offset bookkeeping needed): for a panel at `rect.x = -50` with `pw = 200`, panel-local `tx = 50` is the first column whose destination (`-50 + 50 = 0`) is on-screen, so destination column 0 shows panel-local (source) column 50 - the panel's own right-shifted visible portion, not its raw column 0.

### Behaviors worth knowing

- `draw_panel` computes a safe opaque inner rectangle (the panel interior, inset by `ceil(radius) + 2` px) whenever `panel.alpha >= 1.0`, and passes it to `blit` as `opaque_inner`. Inside that rect the rounded-box SDF is provably `1.0`, so `blit` skips the sqrt-based coverage math there and forces `a = 1.0` directly - a byte-identical fast path. Pixels outside the inner rect (the antialiased corners/edges) still run the full SDF.
- `screen_panel_composites_onto_background` (unit test): a 4x4 red screen (built via `bgra_to_nv12`, then converted back by `composite_into`) placed at (2,2) in an 8x8 blue background leaves the corner blue and the panel interior approximately red - within a few LSBs, since the nv12 round-trip is not bit-exact.
- `disabled_and_degenerate_panels_do_not_panic` (unit test): a camera panel larger than the output and a disabled screen must not trigger out-of-bounds access or panic.
- `blit_opaque_inner_skip_is_byte_identical_to_full_sdf` (unit test): blits the same rounded (r=6) opaque panel via `blit` twice - once with the computed `opaque_inner` rect, once with `None` (full SDF everywhere) - and asserts the two output buffers are byte-identical, including the antialiased corners outside the inner rect.
- `ring_paints_a_band_just_inside_the_camera_edge_and_leaves_center_alone` (unit test): a 20x20 square camera panel with a 3px red ring over a green webcam source - the pixel row/column just inside the edge is strongly ring-tinted (antialiased, not pure - matching the SDF feathering everywhere else in this file) while the panel center stays untouched green.
- `zero_ring_px_leaves_panel_byte_identical_to_no_ring_field` (unit test): `ring_px: 0.0` with a non-black `ring_color` set produces byte-identical output to a panel with no ring fields touched at all - proves the ring never activates on the sentinel value regardless of color.
- `blit_clips_a_negative_origin_instead_of_snapping_to_zero` (unit test): a 200-wide source blitted at `ox = -50` onto a 400-wide destination shows the RIGHT 150 columns of the source at destination columns `[0, 150)` (column 0 == source column 50) and leaves destination columns `>= 150` untouched - the panel's true right edge, not a corner-snapped copy of the whole source.
- `blit_ring_clips_a_negative_origin_the_same_way_as_blit` (unit test): same off-edge scenario through `blit_ring` - the visible portion paints, destination columns beyond the panel's true right edge stay untouched.
- `cover_rect_crops_the_mismatched_axis_for_every_panel_aspect` (unit test): 16:9 source into a square panel crops the sides; into a 16:9 panel it is the whole frame; a square source into a 16:9 panel crops top/bottom; a portrait source into a square panel crops top/bottom; degenerate zero dims clamp instead of dividing by zero.
- `a_wide_webcam_in_a_square_panel_shows_the_centre_not_a_squash` (unit test): a 12x4 webcam striped blue|green|red composited into a 4x4 square panel shows GREEN at the panel centre - the middle third at true proportions, not all three stripes squashed together.

### The screen panel's source rect

`composite_into` draws the whole of `scene.src` into the screen panel - the whole canvas normally, one display switch's fitted rect per span. A mid-take display switch keeps ONE encoder canvas and fits every later frame into it, so the file carries baked black bars from the switch on. The render undoes that by showing only the active SOURCE SPAN's `src` rect (`export::render::spans`). The CPU path passes that rect straight to `resize_crop` (`Source::Crop`); the GPU path normalizes it into `src_min`/`src_max` and the shader samples through it. The raw-copy fast path is gated on the rect being the WHOLE canvas: a span that crops would otherwise be copied out complete with the bars the crop exists to remove.

## select_compositor

```rust
pub fn select_compositor(layout: &Layout) -> Box<dyn Compositor>
```

Returns a `GpuCompositor` when a GPU adapter is available and construction succeeds; otherwise a `CpuCompositor`. Never fails. Public so `render.rs`'s `FrameRenderer` (which serves both the export loop and the preview engine) selects the same compositor once at init and reuses it per frame.
