# src-tauri/src/export/types.rs

Shared value types for the entire export pipeline: colors, geometry, zoom configuration, layout presets, and webcam overlay settings. All types are pure data - no methods beyond `Default` implementations, no I/O, no GPU state.

## Rgb

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }
```

A 24-bit sRGB color.

- `r, g, b: u8` - red, green, blue channels 0..255. *Why:* used in `Background::Gradient` and `Background::Solid`; the background renderer maps these to BGRA pixels.

### Used by

- `src-tauri/src/export/scene/background.rs` - reads `Rgb` fields to fill gradient or solid background buffers.

## FramePoint

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePoint { pub x: i32, pub y: i32 }
```

An integer pixel coordinate in frame-local space (after subtracting monitor origin).

- `x: i32, y: i32` - signed because coordinates near the origin can temporarily go negative during clamped arithmetic. *Why signed:* mouse events captured near a multi-monitor seam can produce raw offsets that are briefly negative before clamping.

### Used by

- `src-tauri/src/export/coordmap.rs` - input and output of all coordinate mapping functions.
- `src-tauri/src/export/camera/mod.rs` - cursor position passed to `CameraSim::step`.
- `src-tauri/src/export/types.rs` - `ZoomRegion.anchor` stores a `FramePoint`.

## RectF

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectF { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
```

A floating-point axis-aligned rectangle in output pixels.

- `x: f32, y: f32` - top-left corner. *Why float:* panel positions are computed from aspect-ratio math that produces non-integer results; rounding only at the blit/SDF boundary preserves accuracy.
- `w: f32, h: f32` - width and height. *Why:* same.

### Used by

- `src-tauri/src/export/scene/mod.rs` - `Panel.rect` is a `RectF`.
- `src-tauri/src/export/gpu/gpu_uniforms.rs` - `build_uniforms` normalizes `RectF` to UV.

## Camera

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera { pub cx: f32, pub cy: f32, pub scale: f32 }
```

The virtual camera state at one frame.

- `cx: f32, cy: f32` - zoom center in output pixels (before zoom). *Why:* the compositors compute a crop rectangle centered here.
- `scale: f32` - zoom multiplier (1.0 = no zoom). *Why:* the crop size is `out / scale`; the compositor resizes this crop back to full output.

### Used by

- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` returns a `Camera`.
- `src-tauri/src/export/gpu/compositor.rs` - `Compositor::composite_into` receives `cam: Camera`.
- `src-tauri/src/export/gpu/gpu_uniforms.rs` - `build_uniforms` derives `zoom_center` and `inv_scale` from it.
- `src-tauri/src/export/coordmap.rs` - `crop` and `project` use `Camera` to compute the zoom transform.

## Easing

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing { Smooth, Linear, Spring { stiffness: f32, damping: f32 }, EaseIn, EaseOut, EaseInOut }
```

Selects the interpolation curve for zoom, layout cross-fade, and camera-move animations.

- `Smooth` - default S-curve (`easing::ease` uses ease-out cubic `1-(1-t)^3`; `camera::ease` uses smoothstep `3t^2-2t^3`). *Why default:* decelerates into the target so it "lands" naturally.
- `Linear` - constant velocity.
- `Spring { stiffness: f32, damping: f32 }` - ease-out-back (a small overshoot past 1, then settle) in `camera::ease`; the params are currently unused.
- `EaseIn` - quadratic accelerate (`t^2`): slow start, fast finish.
- `EaseOut` - quadratic decelerate (`t*(2-t)`): fast start, slow finish.
- `EaseInOut` - quadratic symmetric (`2t^2` up to 0.5, then `1-2(1-t)^2`): slow-fast-slow.

### Used by

- `src-tauri/src/export/easing.rs` and `src-tauri/src/export/camera/mod.rs` - both `ease(e, t)` fns dispatch on this.
- `src-tauri/src/export/types.rs` - `ZoomConfig.easing` and `ZoomRegion.easing` store it.
- `src-tauri/src/export/camera/moves.rs` - camera-move keyframes resolve their `easing` wire-name to this via `easing_from`.

## ZoomConfig

```rust
#[derive(Clone, Copy, Debug)]
pub struct ZoomConfig {
    pub target_scale: f32, pub zoom_in_ms: u32, pub zoom_out_ms: u32, pub idle_release_ms: u32,
    pub clicks_to_trigger: u32, pub merge_window_ms: u32, pub merge_radius_px: u32,
    pub follow_damping: f32, pub dead_zone_px: u32, pub easing: Easing,
}
```

All tuneable parameters for click-zoom behavior. Populated from user settings; keeping them in one struct lets `generate` and `CameraSim::step` be free of magic numbers.

- `target_scale: f32` - peak zoom multiplier. Default: 2.2.
- `zoom_in_ms: u32` - ease-in duration ms. Default: 350.
- `zoom_out_ms: u32` - ease-out duration ms. Default: 450.
- `idle_release_ms: u32` - inactivity gap before auto-zoom-out. Default: 2200.
- `clicks_to_trigger: u32` - clicks required within `merge_window_ms` to start a zoom. Default: 1.
- `merge_window_ms: u32` - time window for multi-click trigger. Default: 600.
- `merge_radius_px: u32` - spatial radius for click merging. Default: 240.
- `follow_damping: f32` - per-frame exponential step size for `CameraSim` (0=instant, higher=slower follow). Default: 0.10.
- `dead_zone_px: u32` - cursor must exceed this distance from center before the camera follows (hold phase). Default: 60. *Note: the actual dead band is computed proportionally from frame size in `CameraSim::step`; this field is reserved for a future per-pixel override.*
- `easing: Easing` - curve for zoom transitions. Default: `Easing::Smooth`.

### Used by

- `src-tauri/src/export/camera/autozoom.rs` - `generate` reads most fields to decide trigger, hold, and release timing.
- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` reads `follow_damping` and `target_scale`.
- `src-tauri/src/export/pipeline/exporter.rs` - receives `cfg` from settings and passes it to both `generate` and `CameraSim::step`.

## ZoomRegion

```rust
#[derive(Clone, Copy, Debug)]
pub struct ZoomRegion {
    pub start_ms: u32, pub end_ms: u32, pub zoom_in_ms: u32, pub zoom_out_ms: u32,
    pub target_scale: f32, pub anchor: FramePoint, pub easing: Easing,
}
```

A single resolved zoom event, baked from either auto-generated click detection or a manual hold in the edit doc. Self-describing so the renderer needs no second lookup.

- `start_ms: u32` - absolute recording time when zoom-in begins. *Why:* `CameraSim::step` tests `t_ms >= start_ms`.
- `end_ms: u32` - absolute time when zoom-out completes. *Why:* the region is active while `t_ms <= end_ms`.
- `zoom_in_ms: u32` - ease-in duration; the zoom-in phase ends at `start_ms + zoom_in_ms`.
- `zoom_out_ms: u32` - ease-out duration; the zoom-out phase begins at `end_ms - zoom_out_ms`.
- `target_scale: f32` - peak zoom multiplier during the hold phase.
- `anchor: FramePoint` - the screen-local pixel that stays centered during zoom-in. *Why:* the anchor is the first click of the trigger cluster (see `autozoom::generate`); anchoring on the first click, not the last, keeps intent stable.
- `easing: Easing` - interpolation curve for this specific region.

### Used by

- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` iterates `&[ZoomRegion]` to find the active region.
- `src-tauri/src/export/camera/autozoom.rs` - `generate` produces `Vec<ZoomRegion>`.
- `src-tauri/src/export/scene/layout.rs` - `anchor_regions` re-maps region anchors into the active panel.

## Background

```rust
#[derive(Clone, Debug)]
pub enum Background {
    Gradient { from: Rgb, to: Rgb, angle_deg: f32 },
    Solid(Rgb),
    Image(PathBuf),
}
```

The background fill behind the screen panel.

- `Gradient { from, to, angle_deg }` - linear gradient. Default: dark navy (36, 41, 56) to purple (88, 64, 120) at 135 degrees.
- `Solid(Rgb)` - flat color fill.
- `Image(PathBuf)` - user-supplied background image. *Why:* allows custom wallpapers without a code change.

Default is `Gradient` matching the embedded `bg.jpg` fallback in `exporter.rs`.

### Used by

- `src-tauri/src/export/scene/background.rs` - renders `Background` to a BGRA pixel buffer for `exporter.rs`.

## Layout

```rust
#[derive(Clone, Copy, Debug)]
pub struct Layout { pub out_w: u32, pub out_h: u32, pub pad_px: u32, pub screen_scale: f32, pub screen_radius_px: f32 }
```

Output canvas and screen panel geometry.

- `out_w: u32, out_h: u32` - output video dimensions in pixels. Default: 3840x2160 (4K).
- `pad_px: u32` - padding between the screen panel edge and the canvas edge. Default: 120 px.
- `screen_scale: f32` - additional scale factor applied to the screen panel (0.1..1.0). Default: 1.0.
- `screen_radius_px: f32` - corner radius for the screen panel. Default: `2160 * 0.016 = 34.56 px`.

### Used by

- `src-tauri/src/export/coordmap.rs` - `inset_rect` and `corner_radius` derive the screen panel position from this.
- `src-tauri/src/export/gpu/compositor.rs` - `Compositor::composite_into` takes `&Layout` for output sizing.
- `src-tauri/src/export/gpu/gpu_uniforms.rs` - `build_uniforms` normalizes rects using `out_w/out_h`.
- `src-tauri/src/export/pipeline/exporter.rs` - one `Layout::default()` per export (currently fixed at 4K).

## OverlayShape

```rust
#[derive(Clone, Copy, Debug)]
pub enum OverlayShape { Circle, Rounded { frac: f32 }, Rect }
```

Shape of the webcam/camera overlay panel.

- `Circle` - radius = `min(w, h) / 2`. Default.
- `Rounded { frac: f32 }` - corner radius = `frac * min(w, h)`. *Why a fraction:* stays proportionate as `size_px` changes.
- `Rect` - no rounding (radius = 0).

### Used by

- `src-tauri/src/export/scene/mod.rs` - `panel_radius` dispatches on this to compute the camera panel radius.

## OverlayPos

```rust
#[derive(Clone, Copy, Debug)]
pub enum OverlayPos { BottomLeft, BottomRight, TopLeft, TopRight, Custom { x: u32, y: u32 } }
```

Anchor corner for the webcam overlay relative to the output canvas.

- `BottomLeft` - default: margins from the bottom-left corner.
- `BottomRight, TopLeft, TopRight` - other corners, each applying the same `margin_x_px` / `margin_y_px`.
- `Custom { x, y }` - absolute pixel coordinates ignoring margins.

### Used by

- `src-tauri/src/export/scene/mod.rs` - `bubble_rect` maps this to a concrete `RectF`.

## OverlayLayout

```rust
#[derive(Clone, Copy, Debug)]
pub struct OverlayLayout {
    pub shape: OverlayShape, pub pos: OverlayPos,
    pub size_px: u32, pub width_px: u32,
    pub margin_x_px: u32, pub margin_y_px: u32, pub enabled: bool,
    pub ring_px: u32, pub ring_color: [u8; 3],
}
```

All overlay (webcam) layout parameters.

- `shape: OverlayShape` - overlay mask shape. Default: `Circle`.
- `pos: OverlayPos` - corner placement. Default: `BottomLeft`.
- `size_px: u32` - camera panel HEIGHT in output pixels. Default: 420.
- `width_px: u32` - camera panel WIDTH in output pixels. Default: 420 (== `size_px`; only diverges when `ModeAppearance.cam_aspect` is `Wide`, giving `round(size_px * 16/9)`). *Why a separate field, not derived at draw time:* the width:height ratio is a per-mode setting (`CamAspect`), so it is resolved once here alongside every other pixel value, the same as `size_px`.
- `margin_x_px: u32` - horizontal inset from the canvas edge. Default: 80.
- `margin_y_px: u32` - vertical inset from the canvas edge. Default: 80.
- `enabled: bool` - whether the overlay is drawn at all. Default: `true`. *Why kept here:* a disabled overlay still contributes its rect for cross-dissolve transitions; the `alpha` in the resolved `Panel` drops to 0 instead.
- `ring_px: u32` - width in output pixels of an optional colored ring/border drawn just inside the panel edge. Default: `0` (no ring). *Why px, not fraction:* resolved once here from `ModeAppearance.cam_ring`'s fraction-of-min-side, same pattern as every other geometry field.
- `ring_color: [u8; 3]` - RGB 0..255 of the ring. Default: `[0, 0, 0]` (unused when `ring_px == 0`).

### Used by

- `src-tauri/src/export/scene/mod.rs` - `resolve` passes `&OverlayLayout` to `bubble_rect` and `panel_radius`, and copies `ring_px`/`ring_color` onto the resolved camera `Panel`.
- `src-tauri/src/settings/appearance.rs` - `overlay_for` constructs this from user settings.
