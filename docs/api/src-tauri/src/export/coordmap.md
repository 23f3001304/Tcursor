# src-tauri/src/export/coordmap.rs

Coordinate mapping utilities that convert between screen space, frame space, composited-base space, panel space, and zoomed-output space. Pure functions with no state - all subsystems (CPU compositor, GPU compositor, FX overlay, cursor renderer) import from here so they agree on pixel placement.

## to_frame

```rust
pub fn to_frame(screen: &ScreenInfo, x: i32, y: i32) -> FramePoint
```

Converts a global screen coordinate `(x, y)` to a frame-local pixel by subtracting the monitor origin.

### Inputs

- `screen: &ScreenInfo` - capture geometry including `origin_x`, `origin_y`. *Why:* on multi-monitor setups global coordinates have an offset; the capture buffer starts at the monitor origin.
- `x: i32, y: i32` - global screen coordinate (e.g. from a mouse event). *Why:* raw events are in global screen space.

### Returns

`FramePoint` with `x = x - screen.origin_x`, `y = y - screen.origin_y`.

### Behaviors worth knowing

- `maps_screen_to_frame_minus_origin` (unit test): origin (100, 50) subtracted from (300, 200) yields (200, 150).

### Used by

- `src-tauri/src/export/autozoom.rs` - converts click coordinates before anchor placement.

## inset_rect

```rust
pub fn inset_rect(sw: u32, sh: u32, layout: &Layout) -> (u32, u32, u32, u32)
```

Returns the screen panel rectangle `(x, y, w, h)` in output pixels: the screen aspect-fitted into the padded area, then scaled by `layout.screen_scale` about the output center.

### Inputs

- `sw: u32, sh: u32` - capture source dimensions. *Why:* the aspect ratio `sw/sh` determines how the screen fits into the available padded area.
- `layout: &Layout` - output dimensions, padding, and `screen_scale`. *Why:* the padded area is `out_w - 2*pad_px` by `out_h - 2*pad_px`; `screen_scale` shrinks the inset further about the center.

### Returns

`(ix, iy, iw, ih)` all as `u32`, rounded to whole pixels. Zero-size inputs are guarded with `max(1)`.

### Behaviors worth knowing

- `to_base_centers_screen_and_insets_corner` (unit test): screen center maps to output center; top-left maps inside the inset, not at the frame corner.
- `inset_scales_about_center_with_screen_scale` (unit test): at `screen_scale=0.5` the inset width halves and the rect re-centers.

### Used by

- `src-tauri/src/export/compositor.rs` - `CpuCompositor::composite_into` uses the crop rect derived from this.
- `src-tauri/src/export/scene.rs` - `resolve` computes the screen panel rect via this function.
- `src-tauri/src/export/gpu_uniforms.rs` - called indirectly through `scene`.

## corner_radius

```rust
pub fn corner_radius(layout: &Layout, iw: u32, ih: u32) -> f32
```

Returns the corner radius for the screen panel: `layout.screen_radius_px` clamped to at most `min(iw, ih) / 2` so the radius never exceeds a half-circle.

### Inputs

- `layout: &Layout` - supplies `screen_radius_px`. *Why:* keeping the radius in the layout struct lets the user's settings drive it without a code change.
- `iw: u32, ih: u32` - inset dimensions. *Why:* the clamp prevents a radius larger than the panel's shortest half-side, which would produce an inverted SDF.

### Returns

`f32` corner radius in output pixels.

### Behaviors worth knowing

- `corner_radius_uses_layout_value_and_clamps` (unit test): normal-sized inset honors the layout value; very small inset (50x60) clamps to 25.

### Used by

- `src-tauri/src/export/scene.rs` - `resolve` and `panel_radius` call this for the screen panel.

## to_base

```rust
pub fn to_base(p: FramePoint, sw: u32, sh: u32, layout: &Layout) -> FramePoint
```

Maps a screen-local frame pixel into the composited base frame (output pixels before zoom) by scaling into the aspect-fitted inset.

### Inputs

- `p: FramePoint` - position within the raw screen capture buffer. *Why:* click and anchor positions come in frame-local pixels.
- `sw: u32, sh: u32` - capture dimensions. *Why:* normalizes `p` to 0..1 before scaling into the inset.
- `layout: &Layout` - provides the inset via `inset_rect`. *Why:* the inset defines the mapping from frame pixels to output pixels.

### Returns

`FramePoint` in output pixels (before any zoom). Screen center maps to output center.

### Used by

- `src-tauri/src/export/autozoom.rs` - converts click anchors to base-frame coords.
- `src-tauri/src/export/layout.rs` - re-anchors zoom regions into the active panel.

## to_panel

```rust
pub fn to_panel(p: FramePoint, sw: u32, sh: u32, rect: RectF) -> FramePoint
```

Maps a screen-local frame pixel into any panel rect (output pixels). Generalizes `to_base` to work with the active layout's screen panel instead of the default inset.

### Inputs

- `p: FramePoint` - position in the raw capture buffer. *Why:* cursor positions come in frame-local pixels.
- `sw: u32, sh: u32` - capture dimensions. *Why:* normalizes `p` to 0..1 before scaling into the panel.
- `rect: RectF` - the active screen panel rect (from `Scene`). *Why:* the layout may be Screen, Camera, or Presenter; each places the screen panel differently.

### Returns

`FramePoint` in output panel pixels. Screen origin maps to `rect.x, rect.y`; screen center maps to the panel center.

### Behaviors worth knowing

- `to_panel_maps_into_rect` (unit test): (960, 540) in a 1920x1080 capture maps to (500, 350) in a rect at (100, 50, 800x600).
- Origin (0,0) maps exactly to the rect origin.

### Used by

- `src-tauri/src/export/exporter.rs` - converts the cursor position each frame.
- `src-tauri/src/export/fx_state.rs` - converts click positions for FX overlay placement.

## crop

```rust
pub fn crop(cam: Camera, ow: u32, oh: u32) -> (f32, f32, f32, f32)
```

Returns `(cx0, cy0, cw, ch)` - the crop rectangle in output pixels that the compositor resizes to fill the full frame, simulating the camera zoom.

### Inputs

- `cam: Camera` - zoom center and scale. *Why:* the crop is centered on `cam.cx/cy` with size `out / scale`.
- `ow: u32, oh: u32` - output dimensions. *Why:* crop size = `ow/scale` x `oh/scale`; origin is clamped to keep the crop inside the frame.

### Returns

`(cx0, cy0, cw, ch)` all as `f32`. `cw` and `ch` are at least 1.0. Origin is clamped so `cx0 + cw <= ow` and `cy0 + ch <= oh`.

### Behaviors worth knowing

- Shared by `CpuCompositor` and the FX overlay so click effects land at the same position as the zoomed image.

### Used by

- `src-tauri/src/export/compositor.rs` - `CpuCompositor::composite_into` passes to `resize_crop`.
- `src-tauri/src/export/coordmap.rs` - `project` calls this to compute the zoom transform.

## project

```rust
pub fn project(bx: f32, by: f32, cam: Camera, ow: u32, oh: u32) -> (f32, f32)
```

Projects a point `(bx, by)` in base output pixels (pre-zoom) to its on-screen position after the camera zoom, matching the crop-and-resize the compositor applies.

### Inputs

- `bx: f32, by: f32` - position in output pixels before zoom. *Why:* FX overlays and cursor drawers compute their positions in base space; `project` maps them to the correct post-zoom screen position.
- `cam: Camera` - zoom parameters. *Why:* the same `crop` call used by the compositor must be applied here.
- `ow: u32, oh: u32` - output dimensions. *Why:* needed for both `crop` and the final scale-to-output step.

### Returns

`(px, py)` in zoomed output pixels. At `scale=1.0` the result equals the input.

### Behaviors worth knowing

- `project_is_identity_at_scale_one` (unit test): (100, 200) at scale 1.0 maps to (100, 200) within 0.5 px.
- `project_maps_crop_corner_to_origin_at_scale_two` (unit test): at scale 2 the crop top-left (480, 270) maps to (0, 0); the crop center (960, 540) maps to the output center.
