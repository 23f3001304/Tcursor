# src/editor/stage/camZoomAction.ts

TS mirror of the webcam-on-zoom action (`apply_cam_zoom_action` + `cam_action_at` in `src-tauri/src/export/scene/mod.rs`, and `ZoomSettings::resolved_cam_action` in `src-tauri/src/settings/model.rs`). The export is the source of truth; this drives the live preview and must match it frame-for-frame.

**Why this file exists at all:** before it, the preview never shrank the webcam during a zoom - the auto-shrink lived only in the export path, so the editor silently disagreed with the rendered video on every zoom. These functions close that gap.

The action resolves in three layers, each overriding the one below: a `camera_moves` keyframe (handled in `frameCam.ts`, and it always wins), then the per-zoom `Zoom.cam_action`, then the global `ZoomSettings` default.

## CamTuple

```ts
export type CamTuple = [number, number, number, number, number, number, number, number, number]
```

`PreviewLayout.cam`: `[x, y, w, h, radius, ringPx, ringR, ringG, ringB]`, all fractions of the preview canvas. *Why per-axis fractions:* `x`/`w` are over the canvas width and `y`/`h` over its height, so scaling about the centre works per-axis while using the identical multiplier the export computes in pixel space.

## zoomProgress

```ts
export function zoomProgress(scale: number, targetScale: number): number
```

TS mirror of the private `zoom_progress`: `0` at `scale` 1.0, `1` at `targetScale`, clamped outside. Shared by both the shrink and the fade so they cannot ride different curves.

## resolvedCamDefault

```ts
export function resolvedCamDefault(zoom: ZoomSettings): CamZoomAction
```

Mirror of `ZoomSettings::resolved_cam_action` - the GLOBAL default.

### Returns

`cam_zoom_default` when set; otherwise derived from the legacy pair: `{shrink:{to:camera_shrink_min}}` when `camera_shrink` is on, else `"stay"`. *Why the fallback:* configs written before `cam_zoom_default` existed must resolve to exactly today's behavior (shrink to 0.62).

## resolveCamAction

```ts
export function resolveCamAction(zooms: Zoom[], tMs: number, fallback: CamZoomAction): CamZoomAction
```

Mirror of `cam_action_at`. The highest-`layer` zoom containing `tMs` supplies the action; ties go to the LAST such zoom, matching Rust's `max_by_key`.

### Behaviors worth knowing

- Overlap resolution deliberately matches `CameraSim`'s highest-layer-wins rule - if they disagreed, the webcam would follow one zoom while the framing followed another.
- Outside every zoom the scale is 1.0, so the returned `fallback` is a no-op whatever it is.

## applyCamZoomAction

```ts
export function applyCamZoomAction(cam: CamTuple, action: CamZoomAction, scale: number, targetScale: number): CamTuple
```

The GEOMETRY half of the mirror. `shrink` scales the panel about its own centre - rect, radius and ring width all by the same smoothstepped factor `m = 1 + (clamp(to, 0.1, 1) - 1) * smoothstep(z)`, exactly like `shrink_camera`. `hide` and `stay` leave geometry untouched.

### Behaviors worth knowing

- `is the identity at no zoom` (vitest): at `m = 1` the result still round-trips through `cx - w/2`, landing ~1e-16 off. The Rust performs the identical recompute, so that is parity, not drift - the test asserts closeness, not bit-equality.
- Ring colour (elements 6-8) is never touched, matching the Rust's `..panel`.

## camZoomAlpha

```ts
export function camZoomAlpha(action: CamZoomAction, scale: number, targetScale: number): number
```

The ALPHA half: the factor to multiply the webcam's draw alpha by. `hide` fades to 0 as the zoom deepens (`1 - smoothstep(z)`, matching the Rust); every other action returns 1.

*Why split from `applyCamZoomAction`:* `PreviewLayout.cam` carries no alpha channel, so the fade cannot be expressed in the tuple. Keeping it a separate pure function lets `hide` be fully tested now and leaves the draw-side plumbing as an isolated change.
