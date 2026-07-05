# src-tauri/src/export/scene.rs

Defines the two-panel scene geometry (screen + camera), resolves each `LayoutId` preset into concrete output-pixel rectangles and radii, and provides cross-dissolve interpolation and zoom-driven camera shrink. Pure CPU geometry - no I/O, no GPU state, identical inputs produce identical outputs.

## Panel

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel { pub rect: RectF, pub radius: f32, pub alpha: f32 }
```

One composited layer expressed as a rounded rectangle in output pixels.

- `rect: RectF` - position and size. *Why:* both compositors (CPU and GPU) use this rect to place and clip the panel.
- `radius: f32` - corner radius in output pixels. *Why:* drives the CPU rounded-box SDF blit and the GPU WGSL shader equally; a circle is expressed as `radius = min(w,h)/2`.
- `alpha: f32` - opacity in 0..1 for cross-dissolve. *Why:* disabled panels (e.g. camera in ScreenOnly mode) keep a valid rect but `alpha=0.0` so compositors can skip drawing without a separate branch.

### Used by

- `src-tauri/src/export/compositor.rs` - `draw_panel` reads `rect`, `radius`, `alpha` to blit and blend.
- `src-tauri/src/export/gpu_uniforms.rs` - `build_uniforms` packs both panels' fields into the WGSL uniform buffer.
- `src-tauri/src/export/exporter.rs` - checks `scene.screen.alpha < 0.5` to disable zoom on CameraOnly layouts.
- `src-tauri/src/export/layout.rs` - `LayoutTrack` stores and interpolates panels during transitions.

## Scene

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene { pub screen: Panel, pub camera: Panel }
```

The complete frame geometry at one instant: screen is the zoomed base layer, camera is the fixed webcam overlay drawn on top.

- `screen: Panel` - captured screen layer. *Why:* the compositor zooms the full base (which includes the screen panel) via a crop; keeping screen separate from camera means the camera is not zoomed.
- `camera: Panel` - webcam overlay. *Why:* drawn after zoom so it stays fixed at its layout-resolved position regardless of zoom depth.

### Used by

- `src-tauri/src/export/compositor.rs` - `CpuCompositor::composite_into` draws screen then camera.
- `src-tauri/src/export/gpu_compositor.rs` - passes both panels to the shader via uniforms.
- `src-tauri/src/export/exporter.rs` - receives scene from `LayoutTrack::scene_at`; applies `shrink_camera` on `scene.camera`.
- `src-tauri/src/export/layout.rs` - produces `Scene` values via `resolve`; interpolates them on transitions.

## Scene::lerp

```rust
pub fn lerp(a: &Scene, b: &Scene, t: f32) -> Scene
```

Component-wise linear interpolation between two scenes for cross-dissolve layout transitions. The caller supplies a pre-eased `t`.

### Inputs

- `a: &Scene` - source scene at t=0. *Why:* the layout track supplies two keyframe scenes bracketing the current time.
- `b: &Scene` - target scene at t=1. *Why:* same; `lerp` does only the blend, not the easing or time lookup.
- `t: f32` - blend factor in 0..1. *Why:* the caller applies the easing curve so `lerp` stays a pure linear operation.

### Returns

A new `Scene` with each `rect` field (`x`, `y`, `w`, `h`), `radius`, and `alpha` of both panels linearly interpolated.

### Behaviors worth knowing

- `lerp_midpoint_is_between` (unit test): at t=0.5 the screen rect width lies exactly halfway between Screen-mode and Camera-mode widths.

## shrink_camera

```rust
pub fn shrink_camera(panel: Panel, scale: f32, target_scale: f32, min: f32) -> Panel
```

Scales the camera panel toward its own center, driven by the current zoom level, so the webcam bubble retreats during zoom-in and returns on zoom-out.

### Inputs

- `panel: Panel` - the camera panel at its normal full size. *Why:* the function returns a modified copy; the caller decides each frame whether to apply it.
- `scale: f32` - current camera zoom scale from `CameraSim`. *Why:* used to compute how far along the zoom-in the camera currently is.
- `target_scale: f32` - the zoom region's full zoom multiplier. *Why:* normalizes `scale` to a 0..1 progress so the shrink is proportionate to depth regardless of the configured max zoom.
- `min: f32` - minimum size multiplier (0.1..1.0, clamped). *Why:* the user setting `camera_shrink_min` controls how small the webcam gets at full zoom.

### Returns

A new `Panel` with rect and radius scaled about the panel center by a smoothstepped multiplier; `alpha` is unchanged.

### Implementation

1. Normalize `z = clamp((scale-1) / max(target_scale-1, 0.001), 0, 1)`.
2. Smoothstep `z` via `t*t*(3-2t)` and compute multiplier `m = 1 + (clamp(min,0.1,1.0) - 1) * smoothstep(z)`.
3. Compute panel center `(cx, cy)`.
4. Scale `w`, `h`, and `radius` by `m`; re-center the rect.

### Behaviors worth knowing

- `shrink_is_identity_at_no_zoom_and_min_at_full` (unit test): at `scale=1.0` the panel is unchanged; at `scale=target_scale=2.2` with `min=0.6` the width shrinks to 120 (60% of 200) and the center stays fixed.

## resolve

```rust
pub fn resolve(id: LayoutId, layout: &Layout, overlay: &OverlayLayout, sw: u32, sh: u32) -> Scene
```

Converts a layout preset ID into a concrete `Scene` in output pixels.

### Inputs

- `id: LayoutId` - which mode to resolve. *Why:* each mode has a distinct spatial arrangement; switching on id here keeps all geometry logic in one place.
- `layout: &Layout` - output dimensions, padding, scale, and corner radius. *Why:* all output-pixel measurements derive from these values.
- `overlay: &OverlayLayout` - webcam shape, corner position, size, and margins. *Why:* the camera panel's rect and radius are entirely determined by overlay settings.
- `sw: u32, sh: u32` - capture source dimensions. *Why:* the screen's aspect ratio must be preserved when fitting it into the inset or computing PiP size.

### Returns

A `Scene` with both panels fully resolved. Disabled panels (`alpha=0.0`) still have valid rects so cross-dissolve transitions remain smooth.

### Implementation

1. Compute the inset rect via `coordmap::inset_rect` and its corner radius via `coordmap::corner_radius`.
2. Compute the bubble rect via `bubble_rect` (inner helper) and its corner radius via `panel_radius` (inner helper).
3. Switch on `id`:
   - `Screen`: screen=inset at alpha=1; camera=corner bubble at alpha=1.
   - `Camera`: camera=centered square of `overlay.size_px` at alpha=1; screen=PiP (30% of inset width, aspect-preserved) at the overlay margin, alpha=1.
   - `Presenter`: side-by-side columns separated by one padding gap; camera fills the left column as a square; screen aspect-fits the right column.
   - `ScreenOnly`: same as `Screen` but camera `alpha=0.0`.
   - `CameraOnly`: same as `Camera` but screen `alpha=0.0`.

### Behaviors worth knowing

- `screen_default_matches_today_inset_and_bubble` (unit test): Screen mode positions the camera at the bottom-left, 80px margins, 420px wide (circle, radius=210).
- `camera_default_is_big_centered_rounded_square` (unit test): Camera mode centers a 1920px-wide webcam with ~4% rounded corners.
- `corner_and_shape_knobs_apply` (unit test): TopRight + Rect shape zeroes the camera radius and shifts it to the right edge.
- `camera_only_disables_screen` (unit test): screen.alpha=0, camera.alpha=1.
- `camera_default_pip_screen_keeps_today_inset` (unit test): PiP screen is inset 80px from bottom-left, not flush.

## layout

Resolves the active `Scene` at any video timestamp from the `SetLayout` action track, cross-fading between presets, and re-anchors zoom regions into the active screen panel. Key items: `LayoutTrack` struct, `LayoutTrack::new(actions, app, ow, oh, sw, sh, transition_ms)`, `LayoutTrack::scene_at(t_ms) -> Scene`, `anchor_regions(raw, track, sw, sh) -> Vec<ZoomRegion>`.

## background

Rasterizes the export background (solid, gradient at any angle, or image stub) into a BGRA pixel buffer. Key items: `render(bg, w, h)` - returns `Vec<u8>` of `w*h*4` BGRA bytes, always deterministic.
