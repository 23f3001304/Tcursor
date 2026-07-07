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
}
```

Common interface for software and hardware compositors.

### Inputs

- `screen: &[u8]` - BGRA bytes of the captured screen frame, `sw * sh * 4` bytes. *Why:* the raw decoded frame is passed by slice to avoid a per-frame copy.
- `sw: u32, sh: u32` - capture source dimensions. *Why:* needed to set up the resize crop.
- `webcam: Option<(&[u8], u32, u32)>` - optional `(bytes, width, height)` for the webcam frame. *Why:* `None` when there is no webcam, or after EOF; the compositor skips the camera panel in that case.
- `cam: Camera` - the virtual camera center and scale for whole-scene zoom. *Why:* drives the crop rectangle that simulates zoom on the base image.
- `bg: &[u8]` - BGRA background image, `out_w * out_h * 4` bytes. *Why:* used as the bottom layer; the screen panel is composited on top of it.
- `layout: &Layout` - output dimensions and padding. *Why:* determines the output buffer size and the inset geometry.
- `scene: &Scene` - the two panels (screen + camera) with rects, radii, and alphas. *Why:* drives placement, clipping, and blend weight for each panel.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller reuse (e.g. pool) the same allocation across frames instead of the compositor allocating fresh each call.

### Returns

Nothing (`()`). The impl resizes `out` to `out_w * out_h * 4` bytes and fully overwrites it with BGRA pixels.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - calls `compositor.composite_into(...)` in the frame loop.
- `src-tauri/src/export/gpu/gpu_compositor.rs` - `GpuCompositor` implements this trait.

## CpuCompositor

```rust
pub struct CpuCompositor;
```

Software compositor with no GPU dependency; zero interior state, trivially `Send + Sync`.

### Implementation of `composite_into`

1. Copy `bg` into a working buffer `base` (`out_w * out_h * 4` bytes).
2. Draw the screen panel onto `base` via `draw_panel`: resize `screen` to `panel.rect` dimensions and alpha-blend with rounded-rect SDF coverage.
3. Compute the zoom crop via `coordmap::crop(cam, ow, oh)` and resize `base` (crop -> full output) into `resized`, simulating the camera zoom. This zooms the entire base including the screen panel but not what will be drawn on top.
4. Clear `out` and copy `resized` into it.
5. Draw the camera panel on top of `out` via `draw_panel`; the camera is not subject to zoom.

### draw_panel / rrect_sd_px / blit_ring (Task 9 Part B - software ring mirror)

`draw_panel` (private) resizes `src` into `panel.rect` and alpha-blends it with rounded-rect SDF coverage (same formula as the GPU shader's `rrect_sd`, factored out here as the private `rrect_sd_px(tx, ty, pw, ph, r) -> f32` helper so both the panel-coverage closure and the ring blend share one SDF implementation). When `panel.ring_px > 0.0`, it then calls the private `blit_ring` to stroke a colored band just inside the panel edge, mirroring the WGSL shader's post-camera-mix ring blend pixel-for-pixel: `band = clamp((ring_px + d) / max(ring_px, 1.0), 0, 1)` for pixels with `-ring_px <= d <= 0`, weighted by the panel's own alpha, blended into the BGRA destination bytes (ring_color is RGB; `dst[0]=B, dst[1]=G, dst[2]=R`).

### Behaviors worth knowing

- `draw_panel` computes a safe opaque inner rectangle (the panel interior, inset by `ceil(radius) + 2` px) whenever `panel.alpha >= 1.0`, and passes it to `blit` as `opaque_inner`. Inside that rect the rounded-box SDF is provably `1.0`, so `blit` skips the sqrt-based coverage math there and forces `a = 1.0` directly - a byte-identical fast path. Pixels outside the inner rect (the antialiased corners/edges) still run the full SDF.
- `screen_panel_composites_onto_background` (unit test): a 4x4 red screen placed at (2,2) in an 8x8 blue background leaves the corner blue and the panel interior red.
- `disabled_and_degenerate_panels_do_not_panic` (unit test): a camera panel larger than the output and a disabled screen must not trigger out-of-bounds access or panic.
- `blit_opaque_inner_skip_is_byte_identical_to_full_sdf` (unit test): blits the same rounded (r=6) opaque panel via `blit` twice - once with the computed `opaque_inner` rect, once with `None` (full SDF everywhere) - and asserts the two output buffers are byte-identical, including the antialiased corners outside the inner rect.
- `ring_paints_a_band_just_inside_the_camera_edge_and_leaves_center_alone` (unit test): a 20x20 square camera panel with a 3px red ring over a green webcam source - the pixel row/column just inside the edge is strongly ring-tinted (antialiased, not pure - matching the SDF feathering everywhere else in this file) while the panel center stays untouched green.
- `zero_ring_px_leaves_panel_byte_identical_to_no_ring_field` (unit test): `ring_px: 0.0` with a non-black `ring_color` set produces byte-identical output to a panel with no ring fields touched at all - proves the ring never activates on the sentinel value regardless of color.

## select_compositor

```rust
pub fn select_compositor(layout: &Layout) -> Box<dyn Compositor>
```

Returns a `GpuCompositor` when a GPU adapter is available and construction succeeds; otherwise a `CpuCompositor`. Never fails. Public so `render.rs`'s `FrameRenderer` (which serves both the export loop and the preview engine) selects the same compositor once at init and reuses it per frame.
