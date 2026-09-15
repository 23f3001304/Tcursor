# src-tauri/src/export/scene/mod.rs

Defines the two-panel scene geometry (screen + camera), resolves each `LayoutId` preset into concrete output-pixel rectangles and radii, and provides cross-dissolve interpolation and zoom-driven camera shrink. Pure CPU geometry - no I/O, no GPU state, identical inputs produce identical outputs.

## Panel

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel { pub rect: RectF, pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }
```

One composited layer expressed as a rounded rectangle in output pixels.

- `rect: RectF` - position and size. *Why:* both compositors (CPU and GPU) use this rect to place and clip the panel.
- `radius: f32` - corner radius in output pixels. *Why:* drives the CPU rounded-box SDF blit and the GPU WGSL shader equally; a circle is expressed as `radius = min(w,h)/2`.
- `alpha: f32` - opacity in 0..1 for cross-dissolve. *Why:* disabled panels (e.g. camera in ScreenOnly mode) keep a valid rect but `alpha=0.0` so compositors can skip drawing without a separate branch.
- `ring_px: f32` - width in output pixels of an optional colored ring/border drawn just inside the panel edge; `0.0` = no ring. *Why on `Panel`, not threaded as a separate parameter:* it needs to ride through `Scene::lerp`/`shrink_camera`/`override_camera` exactly like `radius` does, so both compositors can read it straight off the resolved panel with no extra plumbing. Only ever non-zero on the camera panel - `resolve` always sets the screen panel's ring to `0.0`/`[0,0,0]`.
- `ring_color: [u8; 3]` - RGB 0..255 of the ring; unused when `ring_px == 0.0`.

### Used by

- `src-tauri/src/export/gpu/compositor.rs` - `draw_panel` reads `rect`, `radius`, `alpha` to blit and blend, then `blit_ring` reads `ring_px`/`ring_color` to stroke the border.
- `src-tauri/src/export/gpu/gpu_uniforms.rs` - `build_uniforms` packs both panels' fields (including `scene.camera.ring_px`/`ring_color`) into the WGSL uniform buffer.
- `src-tauri/src/export/pipeline/exporter.rs` - checks `scene.screen.alpha < 0.5` to disable zoom on CameraOnly layouts.
- `src-tauri/src/export/scene/layout.rs` - `LayoutTrack` stores and interpolates panels during transitions.

## Scene

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene { pub screen: Panel, pub camera: Panel, pub src: RectF }
```

The complete frame geometry at one instant: screen is the zoomed base layer, camera is the fixed webcam overlay drawn on top.

- `screen: Panel` - captured screen layer. *Why:* the compositor zooms the full base (which includes the screen panel) via a crop; keeping screen separate from camera means the camera is not zoomed.
- `camera: Panel` - webcam overlay. *Why:* drawn after zoom so it stays fixed at its layout-resolved position regardless of zoom depth.
- `src: RectF` - the sub-rect of the RECORDED CANVAS the screen panel shows, in source pixels. *Why:* A mid-take display switch keeps ONE encoder canvas and fits every later frame into it, so the file carries baked black bars from the switch on. The render undoes that by showing only the active SOURCE SPAN's `src` rect (`export::render::spans`). Both compositors sample the screen texture through it, and every canvas-to-panel map (`coordmap::to_panel`) goes through it, so the cursor, the click effects and the zoom anchors all land on the cropped picture. `full_src(sw, sh)` (the whole canvas) is the value for every take that never switched display.

### Used by

- `src-tauri/src/export/gpu/compositor.rs` - `CpuCompositor::composite_into` draws screen then camera.
- `src-tauri/src/export/gpu/gpu_compositor.rs` - passes both panels to the shader via uniforms.
- `src-tauri/src/export/pipeline/exporter.rs` - receives scene from `LayoutTrack::scene_at`; applies `shrink_camera` on `scene.camera`.
- `src-tauri/src/export/scene/layout.rs` - produces `Scene` values via `resolve`; interpolates them on transitions.

## Scene::lerp

```rust
pub fn lerp(a: &Scene, b: &Scene, t: f32) -> Scene
```

Component-wise linear interpolation between two scenes for cross-dissolve layout transitions. The caller supplies a pre-eased `t`.

`src` does NOT interpolate: it SNAPS to `b`'s, like `ring_color`. A half-way crop rect would slice the bars of neither span; a display-switch transition instead pre-blends the two pictures into `b.src` before compositing (`render::screen_mix`), so one crop rect is all a compositor ever needs.

### Inputs

- `a: &Scene` - source scene at t=0. *Why:* the layout track supplies two keyframe scenes bracketing the current time.
- `b: &Scene` - target scene at t=1. *Why:* same; `lerp` does only the blend, not the easing or time lookup.
- `t: f32` - blend factor in 0..1. *Why:* the caller applies the easing curve so `lerp` stays a pure linear operation.

### Returns

A new `Scene` with each `rect` field (`x`, `y`, `w`, `h`), `radius`, and `alpha` of both panels linearly interpolated.

### Behaviors worth knowing

- `lerp_midpoint_is_between` (unit test): at t=0.5 the screen rect width lies exactly halfway between Screen-mode and Camera-mode widths.

## Scene::with_src

```rust
pub fn with_src(self, src: RectF) -> Scene
```

The same scene showing `src` of the canvas. How `SpanTrack` stamps a span's own source rect onto a `Scene` that `resolve` built from that span's SIZE alone: `resolve` knows the span's width and height (which is what shapes the screen panel) but not where in the canvas the capture was fitted, so the offset is applied here.

### Used by

- `src-tauri/src/export/render/spans.rs` - `SpanTrack::raw`, once per span per frame.

## resolve

```rust
pub fn resolve(id: LayoutId, layout: &Layout, overlay: &OverlayLayout, sw: u32, sh: u32) -> Scene
```

Converts a layout preset ID into a concrete `Scene` in output pixels.

### Inputs

- `id: LayoutId` - which mode to resolve. *Why:* each mode has a distinct spatial arrangement; switching on id here keeps all geometry logic in one place.
- `layout: &Layout` - output dimensions, padding, scale, and corner radius. *Why:* all output-pixel measurements derive from these values.
- `overlay: &OverlayLayout` - webcam shape, corner position, size, width, margins, and ring. *Why:* the camera panel's rect, radius, and ring are entirely determined by overlay settings.
- `sw: u32, sh: u32` - capture source dimensions. *Why:* the screen's aspect ratio must be preserved when fitting it into the inset or computing PiP size.

### Returns

A `Scene` with both panels fully resolved. Disabled panels (`alpha=0.0`) still have valid rects so cross-dissolve transitions remain smooth. Only the camera panel ever carries a non-zero `ring_px`/`ring_color` - the screen panel's are always `0.0`/`[0,0,0]`.

### Implementation

1. Compute the inset rect via `coordmap::inset_rect` and its corner radius via `coordmap::corner_radius`.
2. Compute the bubble rect via `bubble_rect` (inner helper, using `overlay.width_px` for width and `overlay.size_px` for height) and its corner radius via `panel_radius` (inner helper).
3. Switch on `id`:
   - `Screen`: screen=inset at alpha=1; camera=corner bubble (width/height from `overlay.width_px`/`size_px` - aspect-affected) at alpha=1.
   - `Camera`: camera=centered square of `overlay.size_px` at alpha=1 (ALWAYS square - `width_px`/`cam_aspect` is ignored here); screen=PiP (30% of inset width, aspect-preserved) at the overlay margin, alpha=1.
   - `Presenter`: side-by-side columns separated by one padding gap; camera fills the left column as a square (also ignores `width_px`); screen aspect-fits the right column.
   - `ScreenOnly`: same as `Screen` but camera `alpha=0.0`.
   - `CameraOnly`: same as `Camera` but screen `alpha=0.0`.
4. Two panel-building closures: `pan` (screen panel - hardcodes `ring_px: 0.0, ring_color: [0,0,0]`) and `cam_pan` (camera panel - carries `overlay.ring_px as f32, overlay.ring_color`).

### Behaviors worth knowing

- `screen_default_matches_today_inset_and_bubble` (unit test): Screen mode positions the camera at the bottom-left, 80px margins, 420px wide (circle, radius=210).
- `camera_default_is_big_centered_rounded_square` (unit test): Camera mode centers a 1920px-wide webcam with ~4% rounded corners.
- `corner_and_shape_knobs_apply` (unit test): TopRight + Rect shape zeroes the camera radius and shifts it to the right edge.
- `camera_only_disables_screen` (unit test): screen.alpha=0, camera.alpha=1.
- `camera_default_pip_screen_keeps_today_inset` (unit test): PiP screen is inset 80px from bottom-left, not flush.
- `wide_aspect_widens_bubble_and_keeps_right_anchor` (unit test): `Wide` aspect widens the Screen-mode bubble to `round(420 * 16/9)` px while height stays 420, and a `BottomRight`-anchored bubble's x-position correctly uses the WIDENED width, not `size_px`.
- `square_aspect_bubble_rect_matches_pre_task9_shape` (unit test): default (`Square`) bubble rect is still exactly 420x420 - byte-identical to before Task 9.
- `wide_aspect_does_not_affect_big_camera_modes` (unit test): `Wide` on the `camera` mode's `ModeAppearance` has no effect on `Camera` mode's resolved rect - still square.
- `cam_ring_flows_onto_camera_panel_only` (unit test): a `cam_ring` setting produces a non-zero `camera.ring_px`/matching `ring_color`, while `screen.ring_px` stays `0.0`.
- `no_cam_ring_leaves_camera_panel_ring_zero` (unit test): default appearance (no ring) resolves `camera.ring_px == 0.0`.

## layout

Resolves the active `Scene` at any video timestamp from the `SetLayout` action track, cross-fading between presets, and re-anchors zoom regions into each frame's screen panel. Key items: `LayoutTrack` struct, `LayoutTrack::new(actions, app, ow, oh, sw, sh, transition_ms)`, `LayoutTrack::scene_at(t_ms) -> Scene`, `anchor_regions(raw, track, sw, sh) -> Vec<ZoomRegion>`.

## background

Rasterizes the export background (solid, gradient at any angle, or image stub) into a BGRA pixel buffer. Key items: `render(bg, w, h)` - returns `Vec<u8>` of `w*h*4` BGRA bytes, always deterministic.

## arrangement

Pose-based panel arrangements (T34): resolves an `edit::model::Arrangement` into a `Scene` through the camera-keyframe placement machinery (`rect_from_center`/`override_camera`), and derives an `Arrangement` back out of a resolved preset. Key items: `resolve_arrangement(a, base, layout, ov, sw, sh) -> Scene`, `arrangement_of_preset(scene, ow, oh) -> Arrangement`, `pose_of_panel(panel, ow, oh) -> Option<PanelPose>`. Round-tripping a preset through poses reproduces its own pixels to within 0.334 px at 1920x1080.

## cam

The camera-panel functions (`cam_action_at`, `apply_cam_zoom_action`, `shrink_camera`, `rect_from_center`, `override_camera`), re-exported from here - see `docs/api/src-tauri/src/export/scene/cam.md`.
