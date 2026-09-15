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
pub enum Easing { Smooth, Linear, Spring { stiffness: f32, damping: f32, mass: f32 }, EaseIn, EaseOut,
    EaseInOut, Cubic { x1: f32, y1: f32, x2: f32, y2: f32 }, Keys(Keys) }
```

Selects the interpolation curve for zoom, layout cross-fade, and camera-move animations.

- `Smooth` - default S-curve (`easing::ease` uses ease-out cubic `1-(1-t)^3`; `camera::ease` uses smoothstep `3t^2-2t^3`). *Why default:* decelerates into the target so it "lands" naturally.
- `Linear` - constant velocity.
- `Spring { stiffness: f32, damping: f32, mass: f32 }` - a REAL damped harmonic oscillator (`export::spring`, full rationale in `spring.md`): underdamped params overshoot past 1 and ring down, critical and overdamped ones are monotone. The response is remapped onto the spring's own settle time, so `dur_ms` still owns the wall-clock length and the parameters own only the shape. Carried on the wire as `spring(stiffness,damping[,mass])`, exactly like `Cubic`'s `cubic(...)`. *This replaced a fixed ease-out-back curve that ignored both fields* - see `SPRING_DEFAULT` for the look change that came with it.
- `EaseIn` - quadratic accelerate (`t^2`): slow start, fast finish.
- `EaseOut` - quadratic decelerate (`t*(2-t)`): fast start, slow finish.
- `EaseInOut` - quadratic symmetric (`2t^2` up to 0.5, then `1-2(1-t)^2`): slow-fast-slow.
- `Cubic { x1, y1, x2, y2 }` - a user-drawn CSS-semantics cubic bezier through P0=(0,0), P1=(x1,y1), P2=(x2,y2), P3=(1,1); see `export/cubic.md`. Unlike `Spring`, it carries its WHOLE shape, so it survives the round trip through the wire string `cubic(x1,y1,x2,y2)` exactly - it is the only variant `easing_from` can fully reconstruct without falling back to the config's curve. `x1`/`x2` are guaranteed in `[0,1]` by the parser (keeps `x(t)` monotonic); `y1`/`y2` may overshoot.
- `Keys(Keys)` - a curve the motion editor DREW: 2 to 8 keyframes, each with two tangent handles and a per-segment mode (bezier, hold or linear), carried on the wire as `keys(...)` and evaluated by `export::keys::eval`; see `export/keys.md`. Like `Cubic` it carries its whole shape, so it round-trips the wire exactly. Unlike every other variant it can express a plateau mid-ramp and a value that leaves `[0,1]` at a chosen instant, which is what the M3 presets (Snappy, Cinematic, Mechanical) are built out of. The payload is a fixed-size `[Key; 8]` plus a live count, specifically so `Easing` stays `Copy` and `ZoomRegion` keeps its shape - at the cost of making `Easing` a few hundred bytes wide, which is fine because it is copied once per region per frame, never per pixel.

### Where `Key`, `KeyMode` and `Keys` live

In `export/keys.rs`, next to the parser and evaluator that define their clamping rules, and re-exported here (`pub use crate::export::keys::{Key, KeyMode, Keys};`) so every `export::types::` path keeps resolving. *Why not here:* `types.rs` is at its size budget, and the three types are meaningless without the normalisation `parse_keys` applies to them. Their full documentation is `export/keys.md`.

### Used by

- `src-tauri/src/export/easing.rs` and `src-tauri/src/export/camera/mod.rs` - both `ease(e, t)` fns dispatch on this.
- `src-tauri/src/export/types.rs` - `ZoomConfig.easing` and `ZoomRegion.easing` store it.
- `src-tauri/src/export/camera/moves.rs` - camera-move keyframes resolve their `easing` wire-name to this via `easing_from`.

## SPRING_DEFAULT

```rust
pub const SPRING_DEFAULT: Easing = Easing::Spring { stiffness: 100.0, damping: 10.0, mass: 1.0 };
```

What the bare wire word `"spring"` means - the parameters a `Zoom`/`LayoutSeg` that just says `"spring"` gets. This is **Motion's own default spring**: `zeta = 0.5`, a gentle overshoot to ~1.16 that settles without a second bounce.

*Why not react-spring's 170/26, the previous value of this constant:* that is `zeta = 0.997` - critically damped to the eye, no overshoot at all - which would have made `"spring"` indistinguishable from `"smooth"` and silently broken the ZoomInspector's "Punchy" preset and `fromedit_spring_tests::spring_layout_transition_overshoots_the_destination_scene`, which encodes "a spring transition overshoots its destination" as a product property.

*The look DID still change:* the old fixed ease-out-back peaked at 1.100 and this peaks at 1.163, differing by up to 0.38 mid-curve. Any existing doc using `"spring"` renders differently - that is the intended outcome of making the parameters real, and a doc that wants a specific feel can now say so with `spring(stiffness,damping)`.

### Used by

- `src-tauri/src/export/render/fromedit.rs` - `easing_from` returns this for the bare wire-name `"spring"`; a `spring(...)` string carries its own parameters instead (see `render/fromedit.md`).
- `src/shared/math/spring.ts` - `SPRING_DEFAULT` mirrors it, so the preview and the curve cards resolve the bare word to the same oscillator.

## ZoomConfig

```rust
#[derive(Clone, Copy, Debug)]
pub struct ZoomConfig {
    pub target_scale: f32, pub zoom_in_ms: u32, pub zoom_out_ms: u32, pub idle_release_ms: u32,
    pub clicks_to_trigger: u32, pub merge_window_ms: u32, pub merge_radius_px: u32,
    pub follow_damping: f32, pub dead_zone_px: u32, pub easing: Easing,
    pub smoothing_ms: u32,
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
- `dead_zone_px: u32` - cursor must exceed this distance from center before the camera follows (hold phase). Default: 60. *Note: unread. `CameraSim::step` has no dead band any more (a `follow_cursor` region aims at the cursor every step, an anchored one at its anchor); the field is kept for the settings file's shape.*
- `easing: Easing` - curve for zoom transitions. Default: `Easing::Smooth`.
- `smoothing_ms: u32` - settle time, in ms, of the opt-in critically damped post-pass `CameraSim::step` applies to its own output (`camera/smoothing.md`). **Default: 0 = off, and off is bit-identical to the camera before the filter existed** (`jank_probe_tests::filter::smoothing_off_is_bit_identical` pins a fingerprint of all 721 samples of the probe scene). Set from the user-facing settings field `ZoomSettings::camera_smoothing_ms` via `to_zoom_config` (see `settings/model.md`) - a field distinct from `smoothness`, which already means the CURSOR glide (`CursorSettings::smoothness`), and from `follow_damping`, the hold-phase chase rate. Useful values measured on the probe scene: 120ms cuts the worst spike by 72% for ~33ms of lag, 250ms by 86% for ~83ms.

### Used by

- `src-tauri/src/export/camera/autozoom.rs` - `generate` reads most fields to decide trigger, hold, and release timing.
- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` reads `follow_damping` and `target_scale`.
- `src-tauri/src/export/pipeline/exporter.rs` - receives `cfg` from settings and passes it to both `generate` and `CameraSim::step`.
- `src-tauri/src/settings/model.rs` (`ZoomSettings::to_zoom_config`) - sets `smoothing_ms` from the user-facing `camera_smoothing_ms` field.

## ZoomRegion

```rust
#[derive(Clone, Copy, Debug)]
pub struct ZoomRegion {
    pub start_ms: u32, pub end_ms: u32, pub zoom_in_ms: u32, pub zoom_out_ms: u32,
    pub target_scale: f32, pub anchor: FramePoint, pub easing: Easing,
    pub easing_out: Easing,
    pub layer: u32,
    pub cam_action: Option<CamZoomAction>,
    pub follow_cursor: bool,
}
```

A single resolved zoom event, baked from either auto-generated click detection or a manual hold in the edit doc. Self-describing so the renderer needs no second lookup.

- `start_ms: u32` - absolute recording time when zoom-in begins. *Why:* `CameraSim::step` tests `t_ms >= start_ms`.
- `end_ms: u32` - absolute time when zoom-out completes. *Why:* the region is active while `t_ms <= end_ms`.
- `zoom_in_ms: u32` - ease-in duration; the zoom-in phase ends at `start_ms + zoom_in_ms`.
- `zoom_out_ms: u32` - ease-out duration; the zoom-out phase begins at `end_ms - zoom_out_ms`.
- `target_scale: f32` - peak zoom multiplier during the hold phase.
- `anchor: FramePoint` - the screen-local pixel that stays centered during zoom-in. *Why:* the anchor is the first click of the trigger cluster (see `autozoom::generate`); anchoring on the first click, not the last, keeps intent stable. Read only when `follow_cursor` is `false`.
- `easing: Easing` - the ZOOM-IN ramp's curve for this specific region.
- `easing_out: Easing` - the ZOOM-OUT ramp's curve (M3). `CameraSim::step` reads this, and only this, for the ramp back to scale 1 (`export/camera/mod.rs`, the `t_ms >= zout_start` branch); everything else about the region still reads `easing`. *Why a second field rather than a flag:* the two ramps are independent curves in the motion editor's model, and `Easing` is `Copy`, so carrying both costs nothing and keeps every consumer's shape. `regions_from_doc` resolves it from the doc's optional `Zoom::easing_out`, falling back to the ZOOM'S OWN `easing` when absent - never to `cfg.easing` - so a doc written before the field existed renders its out ramp exactly as it always did. Every generated region (`autozoom`, `manual`) sets it equal to `easing`: a split is only ever authored in the editor.
- `layer: u32` - priority when this region overlaps another; higher wins in `CameraSim::step`.
- `cam_action: Option<CamZoomAction>` - per-zoom webcam-on-zoom override carried from `Zoom.cam_action`; `None` inherits the global default. *Why it rides on the region:* `FrameRenderer::step_camera` only has the resolved regions at frame time, so the action must travel with the region it belongs to. `CameraSim` ignores it entirely - it is read only by the camera-panel compositing.
- `follow_cursor: bool` - the zoom-in ramp aims at the LIVE cursor every step instead of the stored `anchor`. *Why:* a `ZoomTarget::Cursor` zoom (every user- or AI-added zoom) has no meaningful stored point - `fromedit::anchor_for` writes screen centre - so easing toward `anchor` zoomed into the middle of the frame and only THEN panned to the cursor once the hold phase took over: a visible two-stage move. It also decouples the aim from the region's timing, so dragging a pill along the timeline re-aims at whatever the cursor is doing at the new time instead of a now-stale point. `false` for `autozoom`/`manual` regions, whose anchor is a real press point the cursor was sitting on.

### Used by

- `src-tauri/src/export/camera/mod.rs` - `CameraSim::step` iterates `&[ZoomRegion]` to find the active region.
- `src-tauri/src/export/camera/autozoom.rs` - `generate` produces `Vec<ZoomRegion>`.
- `src-tauri/src/export/scene/layout.rs` - `anchor_frame` re-maps region anchors into each frame's panel.

## Background

```rust
#[derive(Clone, Debug)]
pub enum Background {
    Gradient { from: Rgb, mid: Option<Rgb>, to: Rgb, angle_deg: f32 },
    Solid(Rgb),
    Image(PathBuf),
}
```

The background fill behind the screen panel.

- `Gradient { from, mid, to, angle_deg }` - linear gradient. `mid` is an optional middle stop that sits at the ramp's midpoint (`t = 0.5`); `None` is the two-stop ramp, bit-identical to what this variant rendered before the field existed. Default: dark navy (36, 41, 56) to purple (88, 64, 120) at 135 degrees, no middle stop.
- `Solid(Rgb)` - flat color fill.
- `Image(PathBuf)` - user-supplied background image. *Why:* allows custom wallpapers without a code change.

Default is `Gradient` matching the embedded `bg.jpg` fallback in `exporter.rs`.

### Used by

- `src-tauri/src/export/scene/background.rs` - renders `Background` to a BGRA pixel buffer for `exporter.rs`.

## Aspect

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Aspect { Source, Wide16x9, Vertical9x16, Square1x1, Classic4x3 }
```

Output frame aspect ratio, chosen in the editor (`EditDoc.aspect`) and applied via `Layout::apply_aspect`. Never crops the capture: the screen aspect-fits inside the frame (`coordmap::inset_rect`) and the background fills the rest - only the frame's own `out_w`/`out_h` change.

- `Source` (default, wire name `"source"`) - the frame adapts to the recording's own dimensions - today's `adapt_to_source` behavior exactly.
- `Wide16x9` (`"wide_16x9"`) - 1920x1080.
- `Vertical9x16` (`"vertical_9x16"`) - 1080x1920.
- `Square1x1` (`"square_1x1"`) - 1080x1080.
- `Classic4x3` (`"classic_4x3"`) - 1440x1080.

### Used by

- `src-tauri/src/edit/model.rs` - `EditDoc.aspect` field.
- `src-tauri/src/edit/ops/api.rs` - `SetAspect` op replaces it.
- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` resolves the output `Layout` from it.
- `src-tauri/src/export/preview/mod.rs` - `with_warm` rebuilds the warm renderer when it changes.

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

## Layout::adapt_to_source

```rust
pub fn adapt_to_source(&mut self, sw: u32, sh: u32)
```

Adapts a still-default (4K) output resolution to match the source video's actual dimensions, so e.g. a 1080p recording exports at 1080p instead of being upscaled to a fixed 4K canvas.

### Inputs

- `sw: u32`, `sh: u32` - probed source video dimensions. *Why:* only consulted when `out_w`/`out_h` are still exactly the 3840x2160 default - a caller that already resolved a different output size (e.g. from `ExportSettings`) is left untouched.*

### Returns

Nothing (`()`) - mutates `self` in place.

### Implementation

1. If `out_w == 3840 && out_h == 2160` (still default) AND `(sw, sh) != (3840, 2160)` (source isn't itself 4K): set `out_w = sw & !1`, `out_h = sh & !1`. The `& !1` evenizes both dimensions, since H.264 requires even width/height.
2. Otherwise a no-op.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` calls this right after probing the source video's dimensions, before building any GPU/layout-dependent state.

## Layout::apply_aspect

```rust
pub fn apply_aspect(&mut self, aspect: Aspect, sw: u32, sh: u32)
```

ONE function mapping an `Aspect` selection to the frame's pixel dimensions - shared by export (`exporter::export`, via `FrameRenderer::new`) and the editor preview (`preview::build_renderer`), so both always agree on the output size for a given source + aspect choice.

### Inputs

- `aspect: Aspect` - the chosen ratio.
- `sw: u32`, `sh: u32` - probed source video dimensions, forwarded to `adapt_to_source` for the `Source` case.

### Implementation

`Source` calls `adapt_to_source(sw, sh)` unchanged (today's behavior). Each fixed preset unconditionally sets `out_w`/`out_h` to its base resolution (long edge 1920), regardless of the source's own dimensions or any prior value - the screen still aspect-fits inside via `inset_rect` (never cropped) and the background fills the new frame.

### Behaviors

- `source_aspect_matches_adapt_to_source_exactly` - `Source` produces the identical `(out_w, out_h)` as calling `adapt_to_source` directly (back-compat guard).
- `fixed_presets_map_to_a_1920_long_edge` - each of the 4 presets yields its documented dimensions regardless of the source's own size.

## Layout::scale_to_long_edge

```rust
pub fn scale_to_long_edge(w: u32, h: u32, max_long: u32) -> (u32, u32)
```

Scales `(w, h)` down (exact ratio preserved) so the long edge is at most `max_long`, evenized (`& !1`) for H.264; never upscales a source already smaller than the budget. Used to pick a cheap preview canvas that matches the export aspect exactly.

### Behaviors

- `scale_to_long_edge_preserves_ratio_and_evenizes` - a 1920x1080 frame capped at 1280 yields exactly 1280x720; a source already under the cap is returned unchanged.

## Layout::resolve

```rust
pub fn resolve(&mut self, aspect: Aspect, resolution: Resolution, sw: u32, sh: u32, preview_cap: Option<u32>)
```

Resolves the final `out_w`/`out_h` for one `FrameRenderer` build: applies `apply_aspect` against the true source dimensions (the RATIO), then `rescale_to_resolution` (`export::settings`, a no-op for `Resolution::Source` - the SIZE), then - when `preview_cap` is `Some(long_edge)` - downscales to that budget via `scale_to_long_edge`, scaling `pad_px`/`screen_radius_px` by the same factor so a preview build stays visually proportional to the full export frame. `preview_cap: None` (export) leaves the aspect+resolution-resolved dimensions untouched. Preview call sites always pass `Resolution::Source`, so this is back-compat identical to the pre-`Resolution` behavior for every existing caller except the export path.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` calls this once per build.

### Behaviors

- `resolve_scales_pad_and_radius_with_the_preview_cap` - a `Wide16x9` + `Resolution::Source` build capped at 1280 yields 1280x720 with `pad_px` scaled by the same 2/3 factor.
- See `src-tauri/src/export/settings.rs`'s `Layout::rescale_to_resolution` for the `Resolution` short-edge convention and its own tests.

