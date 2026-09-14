# src/editor/panels/CameraRingField.tsx

The Camera panel's ring group: an on/off switch and, when on, the ring's width and colour. Split out of `CameraPanel.tsx` so that file stays under its line budget.

## CameraRingField

```tsx
export function CameraRingField({ ring, onChange }: {
  ring: CamRing | null; onChange: (v: CamRing | null) => void;
}): JSX.Element
```

`ring === null` IS "off": the switch writes `DEFAULT_RING` on and `null` off, so there is no separate enabled flag to keep in step with the ring's own fields, and a ring that is off carries no stale width or colour into the export.

The colour row is the shared `Swatches` (`variant="small"`, the default), the same component and the same five colours as `EffectsPanel`'s ripple colour and spotlight tint.

### Look (panel pass, 2026-09-13)

One `.e-grp`: the switch row with the width slider and colour row directly under it. No section heading - the switch's own label is the heading, and a "Ring" head over a "Ring" row is chrome saying the same word twice. This is the pattern for every single-switch group in the panels (`CaptionsPanel` is the other one).

### Used by

- `src/editor/panels/LayoutKnobs.tsx` - the last thing in the Camera group, for whichever layout is being edited (2026-09-14). Nothing about this component changed in the move: it takes a `CamRing | null` and hands one back, and which layout's ring that is has always been the caller's business.
- ~~`src/editor/panels/CameraPanel.tsx`~~ - the original caller, for the `screen` layout only. The webcam's appearance left that panel when the Layouts panel arrived (`CameraPanel.md`), and the ring went with it. Until then, four of the five layouts had a ring in the data model that nothing in the editor could switch on.
