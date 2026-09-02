# src/editor/inspectors/ZoomInspector.tsx

Left-panel inspector for the selected timeline zoom block. Every control applies an
`update_zoom` (or `remove_zoom`/`set_zoom_cam_action`) op via `onApply`, which persists the doc
and bumps the preview.

## CAM_ACTION_OPTIONS

```tsx
export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[]
```

The 4 choices for the "Webcam during zoom" segmented control (Task 26): `Global default`
(`value: null`, inherits `settings.zoom.cam_zoom_default` - see `resolvedCamDefault` in
`stage/camZoomAction.ts`), `Stay`, `Shrink` (`{ shrink: { to: 0.62 } }` - `0.62` mirrors the Rust
`ZoomSettings::camera_shrink_min` default; the `to` fraction itself isn't editable from this
control), and `Hide`. Picking one applies `{ op: "set_zoom_cam_action", id: zoom.id, action: opt.value }`.

## isCamActionSelected

```ts
export function isCamActionSelected(current: CamZoomAction | null | undefined, option: CamZoomAction | null): boolean
```

Whether `option` (one entry of `CAM_ACTION_OPTIONS`) is the one currently in effect for
`current` (`Zoom.cam_action`). `null`/`undefined` both count as "Global default". Any
`{ shrink: {...} }` object counts as the `Shrink` option regardless of its `to` value, since
`Shrink` is the only object-shaped `CamZoomAction` variant and this control never edits `to`
directly.

### Used by

- `ZoomInspector` - drives the segmented control's `on` state and `onClick` payload.

## TargetMode

```ts
export type TargetMode = "cursor" | "region";
```

The two target modes the Target picker offers. `ZoomTarget::Fixed { x, y }` **is** the Region model: the old "Center" button only ever wrote `fixed { 0.5, 0.5 }`, so a doc written by it selects Region with its reticle already at frame centre. There is no `Center` variant in the Rust `ZoomTarget` (only `Cursor` and `Fixed`) and therefore **no migration** - the change is display-level only.

## targetMode

```ts
export const targetMode: (t: ZoomTarget) => TargetMode
```

Reads the stored target back as a mode: `"cursor"` for the cursor-following variant, `"region"` for any stored point.

## targetForMode

```ts
export function targetForMode(mode: TargetMode, current: ZoomTarget): ZoomTarget
```

The target to write when the user picks `mode`. Switching **to** Region keeps whatever point is already stored, so toggling Follow cursor -> Region -> Follow cursor never silently discards an aim the user placed on the stage; a zoom that has only ever followed the cursor starts at frame centre (`{ fixed: { x: 0.5, y: 0.5 } }`).

## zoomScopedSeekMs

```ts
export function zoomScopedSeekMs(nowMs: number, startMs: number, endMs: number): number | null
```

Discoverability fix for a gate finding ("none of these settings work" - live debug traced the wiring as correct; the Target and "Webcam during zoom" controls simply have no visible effect while the playhead sits outside the zoom's own span). `null` while `nowMs` is already inside `[startMs, endMs]` (inclusive both ends - the same span `camZoomAction.resolveCamAction` treats as active, so this never disagrees with what the preview is actually doing); otherwise the span's midpoint, so seeking there puts the just-changed control on screen.

### Behaviors

- `zoomScopedSeekMs (gate finding...)` in `ZoomInspector.test.ts` - no-seek at both inclusive boundaries and mid-span; midpoint (rounded) before/after the span.

### Used by

- `ZoomInspector`'s `seekIntoSpan` - called after every Target/`set_zoom_cam_action` click.

## ZoomInspector

```tsx
export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode, timeMsRef, onSeek }): JSX.Element
```

### Aim-mode props

- `aimMode: boolean` - whether on-stage aiming is currently active (from `Editor`; see `Editor.md`). Drives the `.e-aimbtn` toggle's `on` state and label.
- `moveMode: boolean` - the camera "Move in preview" mode. Aiming and Move mode both claim the same pointer on the same canvas, so the Aim button is **disabled** while Move mode is on, with a title saying to turn it off first.
- `onAimMode: (on: boolean) => void` - toggles aim mode. Also called with `false` when the user picks **Follow cursor**, since a cursor-following zoom has no point to aim.

The "Aim on stage" button only renders while the target is Region - there is nothing to place otherwise.

### Scoped-controls discoverability props

- `timeMsRef: RefObject<number>` - the live playhead, read at click time (not a render prop, matching `EditorPanels`' render-hygiene convention - see its own doc). Feeds `zoomScopedSeekMs`.
- `onSeek: (ms: number) => void` - `Editor`'s `onSeek` (the same path `Timeline`/`Transport` seek through), called by `seekIntoSpan` whenever `zoomScopedSeekMs` returns non-null.
- A one-line `.e-sec-hint` ("Applies while this zoom is active - scrub inside it to preview.") renders above the Target section, ahead of both scoped control groups.
- Every Target button (Follow cursor/Region) and every "Webcam during zoom" option button calls `seekIntoSpan()` after applying its op, jumping the playhead to the zoom's midpoint if it was outside `[start_ms, end_ms]`.

### Transition Curve

Rendered by `<CurveEditor value={zoom.easing} onChange={...} />` (see `CurveEditor.md`) - the six named presets plus, once a handle is dragged, a custom `cubic(x1,y1,x2,y2)`. Rust reconstructs a cubic exactly (`easing_from`), so a hand-drawn zoom curve is identical in preview and export.

