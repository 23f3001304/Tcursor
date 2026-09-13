# src/editor/inspectors/ZoomInspector.tsx

Inspector for the selected timeline zoom block. Every control applies an `update_zoom` (or
`remove_zoom` / `set_zoom_cam_action`) op via `onApply`, which persists the doc and bumps the
preview, so an edit is visible immediately.

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Timing**, **Framing**, **Webcam
during zoom**, **Feel**, then Remove. That is the shared reading order - what it is, when it is, how
it looks, how it moves, then the destructive action (`InspectorShape.md`).

## CAM_ACTION_OPTIONS

```tsx
export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[]
```

The 4 choices for the "Webcam during zoom" row (Task 26): `Global default` (`value: null`, inherits
`settings.zoom.cam_zoom_default` - see `resolvedCamDefault` in `stage/camZoomAction.ts`), `Stay`,
`Shrink` (`{ shrink: { to: 0.62 } }` - `0.62` mirrors the Rust `ZoomSettings::camera_shrink_min`
default; the `to` fraction itself isn't editable from this control), and `Hide`. Picking one applies
`{ op: "set_zoom_cam_action", id: zoom.id, action: opt.value }`.

## isCamActionSelected

```ts
export function isCamActionSelected(current: CamZoomAction | null | undefined, option: CamZoomAction | null): boolean
```

Whether `option` (one entry of `CAM_ACTION_OPTIONS`) is the one currently in effect for `current`
(`Zoom.cam_action`). `null`/`undefined` both count as "Global default". Any `{ shrink: {...} }`
object counts as the `Shrink` option regardless of its `to` value, since `Shrink` is the only
object-shaped `CamZoomAction` variant and this control never edits `to` directly.

### Used by

- `ZoomInspector` - drives the segmented row's `on` state and `onClick` payload.

## TargetMode

```ts
export type TargetMode = "cursor" | "region";
```

The two target modes the picker offers. `ZoomTarget::Fixed { x, y }` **is** the Region model: the old
"Center" button only ever wrote `fixed { 0.5, 0.5 }`, so a doc written by it selects Region with its
reticle already at frame centre. There is no `Center` variant in the Rust `ZoomTarget` (only `Cursor`
and `Fixed`) and therefore **no migration** - the change is display-level only.

## targetMode

```ts
export const targetMode: (t: ZoomTarget) => TargetMode
```

Reads the stored target back as a mode: `"cursor"` for the cursor-following variant, `"region"` for
any stored point.

## targetForMode

```ts
export function targetForMode(mode: TargetMode, current: ZoomTarget): ZoomTarget
```

The target to write when the user picks `mode`. Switching **to** Region keeps whatever point is
already stored, so toggling Follow cursor -> Region -> Follow cursor never silently discards an aim
the user placed on the stage; a zoom that has only ever followed the cursor starts at frame centre
(`{ fixed: { x: 0.5, y: 0.5 } }`).

## zoomScopedSeekMs

```ts
export function zoomScopedSeekMs(nowMs: number, startMs: number, endMs: number): number | null
```

Discoverability fix for a gate finding ("none of these settings work" - live debug traced the wiring
as correct; the Target and "Webcam during zoom" controls simply have no visible effect while the
playhead sits outside the zoom's own span). `null` while `nowMs` is already inside `[startMs, endMs]`
(inclusive both ends - the same span `camZoomAction.resolveCamAction` treats as active, so this never
disagrees with what the preview is actually doing); otherwise the span's midpoint, so seeking there
puts the just-changed control on screen.

### Behaviors

- `zoomScopedSeekMs (gate finding...)` in `ZoomInspector.test.ts` - no-seek at both inclusive
  boundaries and mid-span; midpoint (rounded) before/after the span.

### Used by

- `ZoomInspector`'s `seekIntoSpan` - called after every Target/`set_zoom_cam_action` click.

## PRESETS

```ts
export const PRESETS: { name: string; scale: number; zoom_in_ms: number; zoom_out_ms: number; easing: string }[]
```

The three named feels - **Subtle** (1.6x, 400/500ms, smooth), **Balanced** (2.2x, 350/450ms, smooth)
and **Punchy** (2.8x, 200/300ms, spring). One click writes all four fields in a single `update_zoom`,
which is the "curated first, custom second" disclosure the panel benchmark credits Screen Studio
with; the fields below stay live afterwards, so a preset is a starting point, not a mode.

## activePreset

```ts
export function activePreset(z: Pick<Zoom, "scale" | "zoom_in_ms" | "zoom_out_ms" | "easing">): string
```

The preset a zoom currently matches on **all four** fields (scale within 0.05, the three others
exactly), or `"Custom"`. Drives which plane in the Feel row is selected and what the Feel section's
right-aligned readout says, so nudging Zoom in by 0.05 visibly drops the row to Custom instead of
leaving a preset falsely lit.

## ZoomInspector

```tsx
export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode, timeMsRef, onSeek }): JSX.Element
```

### Sections

**Timing** - `TimingRow` (Start, End) plus the hint that the block is draggable on the timeline. The
header lede carries the span and its length (`spanLede`).

**Framing** - the Scale slider (its value inline on the label row, `2.2x`, tabular), the Target row
(Follow cursor / Region) and, in Region, the Aim on stage toggle. The scoped-controls hint closes the
section: "Applies while this zoom is active, scrub inside it to preview." It sits between the two
scoped groups, Target above and Webcam below, so it reads as covering both.

**Webcam during zoom** - `CAM_ACTION_OPTIONS` as a segmented row.

**Feel** - the three presets as a segmented row, with the matched name as the section's readout, then
Zoom in / Zoom out, then `CurveEditor`. The preset, the two durations and the curve are the four
things a preset writes, so they now sit together instead of three sections apart.

### Aim-mode props

- `aimMode: boolean` - whether on-stage aiming is currently active (from `Editor`; see `Editor.md`).
  Drives the `.e-aimbtn` toggle's `on` state and label.
- `moveMode: boolean` - the camera "Move in preview" mode. Aiming and Move mode both claim the same
  pointer on the same canvas, so the Aim button is **disabled** while Move mode is on, with a title
  saying to turn it off first.
- `onAimMode: (on: boolean) => void` - toggles aim mode. Also called with `false` when the user picks
  **Follow cursor**, since a cursor-following zoom has no point to aim.

The "Aim on stage" button only renders while the target is Region - there is nothing to place
otherwise. It is a plane now, not an outlined button, and its active state tints with
`--insp-accent` (the zoom lane's violet) rather than drawing a colored border.

### Scoped-controls discoverability props

- `timeMsRef: RefObject<number>` - the live playhead, read at click time (not a render prop, matching
  `EditorPanels`' render-hygiene convention). Feeds `zoomScopedSeekMs`.
- `onSeek: (ms: number) => void` - `Editor`'s `onSeek`, called by `seekIntoSpan` whenever
  `zoomScopedSeekMs` returns non-null.
- Every Target option and every "Webcam during zoom" option calls `seekIntoSpan()` after applying its
  op, jumping the playhead to the zoom's midpoint if it was outside `[start_ms, end_ms]`.

### Transition curve

`CurveEditor` (see `CurveEditor.md`) - the six named curves as a segmented row plus one canvas whose
handles shape a cubic directly. Rust reconstructs a cubic exactly (`easing_from`), so a hand-shaped
zoom curve is identical in preview and export.
