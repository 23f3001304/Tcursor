# src/editor/inspectors/zoomInspectorModel.ts

The Zoom inspector's pure model - everything `ZoomInspector.tsx` decides before it renders: how a zoom's target reads and round-trips (`targetMode`/`targetForMode`), the per-zoom webcam action table, the seek-into-span rule, the smart-typing duration options, and the `GraphInput` the Motion section's graph is drawn from. Split out of `ZoomInspector.tsx` so that file is only the panel, and so `zoomInspectorModel.test.ts` can pin these rules without a DOM.

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

## CAM_ACTION_OPTIONS

```tsx
export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[]
```

The 4 choices for the "Webcam during zoom" row (Task 26): `Global default` (`value: null`, inherits
`settings.zoom.cam_zoom_default` - see `resolvedCamDefault` in `stage/camera/camZoomAction.ts`), `Stay`,
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

## durationOptions

```ts
export function durationOptions(smart: boolean): SegOption[]
```

The Timing row's duration switch, as `SegRow` options: **Fixed** (the end stays where you put it) and **Smart typing** (the end follows the typing after the start), whichever is `on`. Picking Smart sends `update_zoom { smart_typing: true }`; the backend refits `end_ms` from `typing.json` then and on every later start move (`ops::smart_zoom`), so nothing here computes a time. Test: `durationOptions (smart typing duration)`.

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

## zoomGraphInput

```ts
export function zoomGraphInput(zoom: Zoom, zooms: Zoom[]): GraphInput
```

The zoom as the graph draws it: lane `"zoom"`, the span, `peak` = `scale`, the in ramp from `easing` /
`zoom_in_ms`, the out ramp from `easing_out ?? easing` / `zoom_out_ms`, `followHint` when the target
follows the cursor, and the nearest zoom before and after (from `zooms`, the whole lane, which
`PropertiesSlot` passes as `doc.zooms`) as the ghosts.
