# src-tauri/src/export/scene/cam.rs

The camera panel: how a zoom region's `CamZoomAction` and a manual `CamPose` change the camera rectangle that `resolve` laid out. Split out of `scene/mod.rs`, which keeps `Panel`, `Scene` and the layout resolver; `mod.rs` re-exports every function here, so callers still say `export::scene::shrink_camera`. Its two test files, `cam_tests.rs` and `action_tests.rs`, moved with it.

## cam_action_at

```rust
pub fn cam_action_at(regions: &[ZoomRegion], zoom: &ZoomSettings, out_t: u32) -> (CamZoomAction, f32)
```

Resolves which webcam-on-zoom action - AND at what peak scale - is in force at a given output time - the three-layer lookup (per-zoom override, then global default) collapsed into one pure function so it can be unit-tested without building a `FrameRenderer`.

**Returns the winning region's own `target_scale`, not the global one.** `target_scale` is a first-class per-zoom slider (presets 1.6/2.2/2.8, `ZoomRegion::target_scale`), and `apply_cam_zoom_action` divides by whatever scale it is handed (`zoom_progress`) - so before this, every region was fed the GLOBAL `zoom.target_scale` regardless of its own slider value: a 1.6x zoom only ever reached ~50% progress against a 2.2x global default (`Hide` left the webcam half-visible; `Shrink` never reached its floor). Because `ZoomRegion::target_scale` is never optional (unlike `cam_action`, which a region may leave unset), ANY active region reports its own scale even when its ACTION inherits the global default - only when NO region is active does `target_scale` fall back to `zoom.target_scale`.

### Inputs

- `regions: &[ZoomRegion]` - the resolved zoom regions. *Why the regions and not the `EditDoc`:* `step_camera` only has regions at frame time.
- `zoom: &ZoomSettings` - supplies the fallback ACTION via `resolved_cam_action` and the fallback SCALE via `zoom.target_scale`.
- `out_t: u32` - output-time milliseconds (`t - video_start`), the same clock the regions are stored in.

### Returns

`(CamZoomAction, f32)` - the `cam_action` of the highest-`layer` region containing `out_t` (or the global default when that region inherits `None` or no region is active), paired with that SAME region's `target_scale` (or `zoom.target_scale` when no region is active).

### Behaviors worth knowing

- `highest_layer_region_supplies_the_action_when_zooms_overlap` (unit test): overlap resolution deliberately matches `CameraSim::step`'s highest-layer-wins rule - if they disagreed, the webcam would follow one zoom while the framing followed another.
- `cam_action_falls_back_to_the_global_default` (unit test): outside every region the zoom scale is 1.0, so the returned action is a no-op regardless of which one it is.
- `cam_action_at_returns_the_winning_regions_own_scale_not_the_global_one` / `cam_action_at_falls_back_to_the_global_scale_when_no_region_is_active` (unit tests, `action_tests.rs`): the scale half of the return value, isolated from the action half.
- `per_zoom_scale_reaches_full_effect_at_its_own_peak_not_the_global_one` (unit test): feeding `apply_cam_zoom_action` the region's own scale (1.6) at a live scale of 1.6 reaches `alpha == 0` (full `Hide`) - the actual bug this fixes, not just the returned number.

## apply_cam_zoom_action

```rust
pub fn apply_cam_zoom_action(panel: Panel, action: CamZoomAction, scale: f32, target_scale: f32) -> Panel
```

Applies the resolved webcam-on-zoom action to the camera panel. This is the single seam every zoom-driven camera behavior goes through, so the export and the TS preview mirror (`src/editor/stage/camera/camZoomAction.ts`) only have one function to agree on.

### Inputs

- `panel: Panel` - the camera panel at its resolved size. *Why:* returns a modified copy; the caller decides per frame whether to apply it.
- `action: CamZoomAction` - the resolved action (`zoom.cam_action` falling back to `ZoomSettings::resolved_cam_action`). *Why resolved by the caller:* resolution needs the active zoom region and the settings, neither of which this pure function should reach for.
- `scale: f32` - current camera zoom scale from `CameraSim`.
- `target_scale: f32` - the zoom region's full zoom multiplier. *Why both:* together they give the 0..1 progress that drives every action's curve.

### Returns

`Shrink { to }` delegates to `shrink_camera(panel, scale, target_scale, to)`; `Hide` returns the panel with `alpha` multiplied by `1 - smoothstep(z)`; `Stay` returns `panel` unchanged.

### Behaviors worth knowing

- `shrink_action_is_identical_to_shrink_camera` (unit test): because `Shrink` delegates rather than reimplementing, a default-resolved action reproduces the pre-action behavior at every sampled scale - the byte-identity guard for existing docs.
- `hide_fades_alpha_out_as_the_zoom_deepens` (unit test): `Hide` only ever touches `alpha`; rect, radius and ring width are left alone.
- Keyframes win: `FrameRenderer::step_camera` skips this call entirely when a `camera_moves` keyframe is active, so a keyframed camera is never also shrunk.

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

A new `Panel` with rect, radius, and ring width scaled about the panel center by a smoothstepped multiplier; `alpha` and `ring_color` are unchanged.

### Implementation

1. Normalize `z = clamp((scale-1) / max(target_scale-1, 0.001), 0, 1)`.
2. Smoothstep `z` via `t*t*(3-2t)` and compute multiplier `m = 1 + (clamp(min,0.1,1.0) - 1) * smoothstep(z)`.
3. Compute panel center `(cx, cy)`.
4. Scale `w`, `h`, `radius`, and `ring_px` by `m`; re-center the rect.

### Behaviors worth knowing

- `shrink_is_identity_at_no_zoom_and_min_at_full` (unit test): at `scale=1.0` the panel is unchanged (including `ring_px`); at `scale=target_scale=2.2` with `min=0.6` the width shrinks to 120 (60% of 200), `ring_px` shrinks by the same 0.6x, and the center stays fixed.

## rect_from_center

```rust
pub fn rect_from_center(p: CamPose, ow: f32, oh: f32, aspect: f32) -> RectF
```

Converts a sampled `CamPose` (Task 4's camera_moves interpolator output - center `x`/`y` + `size`, all fractions of the output frame) into the camera panel's `RectF`. Used by `override_camera` (below), which is what `FrameRenderer::step_camera` (`src-tauri/src/export/render/mod.rs`) actually calls to override the scene's static camera-panel `Panel` when `CameraMoveTrack::sample` returns `Some`.

### Inputs

- `p: CamPose` - `x`/`y` are the PiP's center as a fraction of the frame; `size` is its height as a fraction of the frame. *Why fractions, not pixels:* matches `CameraMove`'s wire units and stays resolution-independent.
- `ow: f32`, `oh: f32` - output canvas dimensions in pixels. *Why:* converts the fractional pose to the pixel `RectF` the panel/compositor need.
- `aspect: f32` - the panel's own width/height IN PIXELS (`1.0` square, `16/9` for a `CamAspect::Wide` bubble). *Why the caller supplies it:* a `CamPose` carries height only, so width has to come from somewhere; the renderer reads it straight off the pre-override `scene.camera.rect` (`rect.w / rect.h`), which `resolve`/`bubble_rect` already sized from `overlay_for`'s `width_px`/`size_px`. Without it a single `camera_moves` keyframe silently squared a Wide panel for the rest of the clip. Clamped to `>= 0.01` so a degenerate zero-height static panel can't produce a zero-width rect.

### Returns

`RectF { x, y, w, h }` (top-left form): `h = p.size * oh`; `w = h * aspect`; `x = p.x * ow - w/2`, `y = p.y * oh - h/2` so the rect stays centered at `(p.x * ow, p.y * oh)` at any aspect. `aspect = 1.0` reproduces the old square exactly.

### Behaviors worth knowing

- `rect_from_center_is_squared_and_centered` (unit test): `CamPose { x: 0.5, y: 0.5, size: 0.3 }` at `1920x1080` with `aspect = 1.0` yields a rect whose height and width both equal `0.3 * 1080` and whose center lands exactly at `(960, 540)`.
- `rect_from_center_keeps_a_wide_panel_wide` (unit test): the same pose at `aspect = 16/9` keeps `h = size * oh` but yields `w / h == 16/9`, still centered on the pose in both axes.
- `static_cam_pose_round_trips_a_wide_rect` (unit test): `static_cam_pose(rect) -> rect_from_center(..., rect.w / rect.h)` reproduces a 16:9 bubble exactly - the aspect is what makes the two functions true inverses for a non-square panel.

## override_camera

```rust
pub fn override_camera(panel: Panel, p: CamPose, ow: f32, oh: f32, aspect: f32) -> Panel
```

Task 9 Part C, plus keyframe shapes (2026-09-14). Applies a `camera_moves` keyframe override to a resolved camera `Panel`: replaces its rect via `rect_from_center`, sets its `radius` from the pose's own shape when it has one (`round`, a fraction of the NEW short side - a keyframed circle, rounded rect or rect, and the morph between them) or else scales the static `radius` by the height ratio, and scales `ring_px` by that ratio either way - so a circle (`radius == min(w,h)/2` at its STATIC size) stays a true circle and its ring stays proportionate after a keyframe grows or shrinks the panel. Fixes the pre-Task-9 bug where only the rect was replaced, leaving `radius` at the panel's static value and distorting a circle into an ellipse-cropped-to-a-circle-mask look whenever a keyframe resized it.

### Inputs

- `panel: Panel` - the scene's currently-resolved camera panel (static size, from `resolve`). *Why take the whole `Panel`, not just its radius:* needs `rect.h` (the pre-override height) to compute the scale ratio, and preserves `alpha`/`ring_color` untouched via struct update syntax.
- `p: CamPose` - the sampled camera-move keyframe pose (center + new height fraction + optional corner fraction).
- `ow: f32`, `oh: f32` - output canvas dimensions in pixels, forwarded to `rect_from_center`.
- `aspect: f32` - the panel's own w/h, forwarded to `rect_from_center` so a Wide (16:9) panel keeps its shape through the override. The caller derives it from THIS panel's pre-override rect, so the two can never disagree.

### Returns

A new `Panel` with `rect = rect_from_center(p, ow, oh, aspect)`, `radius = round * min(rect.w, rect.h)` when `p.round` is `Some` else `panel.radius * (new_h / old_h.max(0.001))`, `ring_px = panel.ring_px * (new_h / old_h.max(0.001))`, and `alpha`/`ring_color` copied from `panel` unchanged (`old_h.max(0.001)` guards a degenerate zero-height static panel from dividing by zero). The two radius rules agree whenever the pose's `round` IS the static panel's own fraction (`static_cam_pose`), which is what a `"layout"` keyframe inherits - so an old doc renders as it always did.

### Behaviors worth knowing

- `override_camera_keeps_circle_round_after_resize` (unit test): a 200x200 circle panel (`radius: 100`) overridden to a `CamPose` that resolves to 100x100 yields `radius: 50` - still exactly `min(w,h)/2`.
- `override_camera_grows_radius_when_panel_grows` (unit test): a 100x100 panel (`radius: 50`, `ring_px: 4`) overridden to resolve 4x larger yields `radius: 200` and `ring_px: 16` - both scaled by the same 4x; `ring_color` and `alpha` are untouched.
- `override_camera_matches_rect_from_center_position` (unit test): the returned `rect` is identical to calling `rect_from_center` directly - only `radius`/`ring_px` differ from a naive rect-only replacement.
- `override_camera_takes_the_poses_own_shape_when_it_has_one` (unit test, `cam_tests.rs`): a 100x100 panel with a 12 px radius posed at 400 px: `round 0.5` -> `radius 200`, `round 0` -> `0`, a Wide 16:9 pose at `round 0.25` -> a quarter of the SHORT side (`50`, not the width) with the ring still scaled by the height ratio, and no `round` -> the static 12 px scaled 4x (`48`).

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::step_camera`'s `camera_moves` override block calls this instead of replacing `scene.camera.rect` directly.
