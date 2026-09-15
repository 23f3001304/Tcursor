# src/editor/inspectors/ZoomInspector.tsx

Inspector for the selected timeline zoom block. Every control applies an `update_zoom` (or
`remove_zoom` / `set_zoom_cam_action`) op via `onApply`, which persists the doc and bumps the
preview, so an edit is visible immediately.

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Framing**, **Timing**, **Motion**,
**Webcam during zoom**. Delete is in the header, not last (`InspectorShape.md`).

**Why this order (owner, 2026-09-14).** The brief is "Zoom / 5.59s to 9.58s / Scale 2.8x / Target
Follow cursor or Region / Motion Snappy, Soft, Cinematic" - the header answers when, so the first
section is free to answer the thing a zoom is actually for: how close it gets and what it aims at.
Timing then holds the four numbers that shape the move, Motion is the curve those numbers run on,
and the webcam override is last because it is the only section that is about a different
object.

## TargetMode

```ts
export type TargetMode = "cursor" | "region";
```

The two target modes the picker offers. `ZoomTarget::Fixed { x, y }` **is** the Region model: the old
"Center" button only ever wrote `fixed { 0.5, 0.5 }`, so a doc written by it selects Region with its
reticle already at frame centre. There is no `Center` variant in the Rust `ZoomTarget` (only `Cursor`
and `Fixed`) and therefore **no migration** - the change is display-level only.

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

**Motion** (M3, replacing Feel on 2026-09-15) - `MotionField` (`motion/MotionField.md`): the preset row over the graph of
the zoom's whole motion, scale rising over the in ramp, the hold, the fall over the out ramp, with
the neighbouring zooms' handoffs as ghosts. A preset writes both ramps' curves; a key drag writes the
ramp it is on; dragging a ramp's end key retimes `zoom_in_ms` / `zoom_out_ms`, the same fields the
Timing row edits, so the two agree by construction. The section's readout says "Custom" only while
no preset is lit (`motionReadout`). The old Feel row bundled a curve with two durations under a
name; the durations now stay in Timing and the name is honest about the one thing it sets.

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

### Motion graph

`MotionGraph` samples the zoom's curves through the same `ease()` the preview runs, and Rust parses the
same `keys(...)` string (`export/keys.md`), so a hand-drawn curve is identical in preview and export.
