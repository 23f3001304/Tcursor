# src/editor/inspectors/ZoomInspector.tsx

Inspector for the selected timeline zoom block. Every control applies an `update_zoom` (or
`remove_zoom` / `set_zoom_cam_action`) op via `onApply`, which persists the doc and bumps the
preview, so an edit is visible immediately.

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Framing**, **Timing**, **Feel**,
**Webcam during zoom**. Delete is in the header, not last (`InspectorShape.md`).

**Why this order (owner, 2026-09-14).** The brief is "Zoom / 5.59s to 9.58s / Scale 2.8x / Target
Follow cursor or Region / Feel Subtle, Balanced or Punchy" - the header answers when, so the first
section is free to answer the thing a zoom is actually for: how close it gets and what it aims at.
Timing then holds the four numbers that shape the move, Feel is the one-click version of two of
them, and the webcam override is last because it is the only section that is about a different
object.

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

## ZoomInspector

```tsx
export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode, timeMsRef, onSeek }): JSX.Element
```

### Sections

**Framing** - the scale as a 30px tabular hero (`2.8x`, unit dimmed and small) with its slider
directly underneath and no label row of its own: the number IS the label. Then the Target row
(Follow cursor / Region) and, in Region only, the Aim on stage toggle. The scoped-controls hint
closes the section: "Applies while this zoom is active, scrub inside it to preview."

**Timing** - one `ValueRow` of four cells, **Start / End / In / Out**, on a single raised plane, with the
span's length against the clip's as the section's right-aligned readout (`2.60s of 10s`). Start is
clamped to the zoom's end; In and Out are each clamped to the span. Under the cells sits a labelled
**Duration** row (`durationOptions`, Fixed / Smart typing, the same `.e-fl` label idiom as Target -
an unlabelled pair of buttons read as orphaned in the owner's screenshot); the hint names the two things the
timeline does that this row does not, or - for a smart zoom - says that the end lands one hold
after the last key of the typing that starts there and refits whenever the start moves.

End is a typed field like the other three (a first draft left it to the pill's right edge alone; the owner-facing rule is that a value you can read is a value you can type).

**Feel** - `FEEL_PRESETS` as a segmented row (`zoomFeel.md`). The section's readout says "Custom"
only while nothing is lit: with a preset matched the row already names it, and repeating it on the
heading row would be the same word twice.

Under the row, the whole custom-curve apparatus - `CurveEditor`, which is the named-curve row, the
drag-the-dots canvas and, for a spring, `SpringControls` - is folded into one quiet `Disclosure`
labelled "Custom". Curated first, custom second: the row is three clicks that cover the common
cases, and the panel stays short until someone actually wants to shape a cubic.

**Webcam during zoom** - `CAM_ACTION_OPTIONS` as a segmented row.

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

`CurveEditor` (see `CurveEditor.md`), inside Feel's "Custom" disclosure - the six named curves as a
segmented row plus one canvas whose handles shape a cubic directly. Rust reconstructs a cubic exactly
(`easing_from`), so a hand-shaped zoom curve is identical in preview and export.
