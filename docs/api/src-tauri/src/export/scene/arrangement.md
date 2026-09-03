# src-tauri/src/export/scene/arrangement.rs

Pose-based panel arrangements (T34): the two-way bridge between an `edit::model::Arrangement` and the `Scene` the compositors draw. Pure CPU geometry, no I/O - and deliberately built on the machinery that already existed rather than a second placement path, because the whole design rests on one claim: **a preset converted to poses and resolved back is the same picture the preset was.** `resolve_arrangement` places panels with `rect_from_center`/`override_camera` (the camera-keyframe path verbatim), so a pose can only ever move and scale a panel, never stretch it, and the radius/ring/shape rules stay wherever the presets already had them.

## SHOWN_ALPHA

```rust
const SHOWN_ALPHA: f32 = 0.004;
```

The alpha above which a resolved panel counts as visible, so `pose_of_panel` maps a hidden one to `None`. Matches the preview's own draw threshold (`toPreviewLayout` in `src/editor/timeline/layoutTrack.ts`) so both sides agree on "not shown"; a preset-resolved panel is only ever exactly `0.0` or `1.0` anyway, and this only matters if a mid-cross-fade scene is ever fed in.

## pose_of_panel

```rust
pub fn pose_of_panel(p: &Panel, ow: f32, oh: f32) -> Option<PanelPose>
```

One resolved `Panel` expressed as a normalized pose, or `None` when the panel is not shown (`alpha <= SHOWN_ALPHA`).

### Inputs (what, and why it is needed)

- `p: &Panel` - the resolved panel, in output pixels. *Why:* poses are derived FROM resolution, never guessed alongside it, so there is one definition of where a preset puts a panel.
- `ow` / `oh` - output frame width/height in pixels. *Why:* the divisor that makes the pose resolution-independent.

### Returns

`Some(PanelPose)` with `cx`/`cy` = the panel's center over `ow`/`oh` and `size` = `rect.h / oh`; `None` for a hidden panel. The panel's WIDTH is intentionally discarded - it is re-derived from the panel's aspect on the way back, which is what makes the round trip incapable of stretching content.

## arrangement_of_preset

```rust
pub fn arrangement_of_preset(s: &Scene, ow: f32, oh: f32) -> Arrangement
```

The arrangement equivalent of an already-resolved preset `Scene` - the "preset as a one-click starting point" conversion, and the payload the editor feeds a `SetArrangement` op when the user picks a preset for a segment they then want to drag.

Both panels go through `pose_of_panel`, so a preset that hides a panel (`ScreenOnly`'s cam, `CameraOnly`'s screen) converts to a `None` for it and the converted segment shows exactly what the preset showed.

**Parity, measured.** `resolve_arrangement(arrangement_of_preset(p)) == resolve(p)` to within **0.334 px** across all five presets at 1920x1080 on the default appearance, and **0.111 px** on a fully re-styled one (`arrangement_tests.rs`, tolerance 0.5 px). The residual is not error in this conversion: `coordmap::inset_rect` ROUNDS the screen inset to whole pixels (1707 px wide at 1920x1080), so the preset's screen rect is a third of a pixel wider than the exact source aspect a pose re-derives.

**Known divergence - a Wide webcam on the big-camera presets.** `scene::resolve` draws `Camera`, `CameraOnly` and `Presenter`'s webcam as a `centered_square`, ignoring `OverlayLayout::width_px` entirely (only `bubble_rect` honors it). An arrangement always honors the appearance-configured shape, so under `CamAspect::Wide` a converted big-camera preset comes out 16:9 where the preset itself was square - same height, same center, wider. Pinned by `a_wide_webcam_widens_the_big_camera_presets_which_resolve_draws_square` rather than papered over; the presets' own square is the pre-existing quirk, and changing it would change how those presets render.

## resolve_arrangement

```rust
pub fn resolve_arrangement(a: &Arrangement, base: Scene, layout: &Layout, ov: &OverlayLayout,
                           sw: u32, sh: u32) -> Scene
```

Resolve an arrangement into the `Scene` the compositors draw - the arrangement-over-preset half of the T34 model.

### Inputs (what, and why it is needed)

- `a: &Arrangement` - the poses to place. *Why:* the segment's own composition; it WINS over the preset.
- `base: Scene` - the segment's PRESET-resolved scene (from its `layout` name). *Why:* everything a pose does not carry comes from where the preset got it - the cam's shape/radius/ring basis, and the rect a hidden panel keeps so a cross-dissolve to it still slides.
- `layout: &Layout` - output dims + the screen corner-radius rule. *Why:* the posed screen panel's radius must follow the same `coordmap::corner_radius` rule every preset's screen panel does (a fraction of canvas height, clamped to half the panel's min side) - it does NOT scale with the panel.
- `ov: &OverlayLayout` - the appearance-configured webcam shape. *Why:* supplies the cam panel's own aspect (`width_px / size_px`, i.e. 16:9 under `CamAspect::Wide`), which is what the pose's height is multiplied by.
- `sw` / `sh` - captured screen dimensions. *Why:* the screen panel's own aspect, so a posed screen letterboxes its source exactly as `inset_rect` does.

### Returns

A `Scene`. For each panel: `Some(pose)` places it at `alpha = 1.0` (so a posed panel is shown even when the base preset hid it - a `CameraOnly` arrangement CAN give the screen a pose); `None` keeps `base`'s rect at `alpha = 0.0`.

### Implementation

The screen panel is `rect_from_center(pose, ow, oh, sw/sh)` with `corner_radius` re-evaluated for the new size and no ring (the screen panel never has one). The camera panel is `override_camera(base.camera, pose, ow, oh, width_px/size_px)` - literally the function a `camera_moves` keyframe uses - which re-centers the rect and scales the static radius and ring width by the height ratio, so a circle stays a circle and the ring keeps its proportion at any size. That ratio is exactly `1.0` when the pose reproduces the preset, which is why the parity above holds to a fraction of a pixel instead of approximately.

### Used by

- `src-tauri/src/export/scene/layout.rs` - `LayoutTrack::from_segs`, the single place a `LayoutSeg` becomes a `Scene`
