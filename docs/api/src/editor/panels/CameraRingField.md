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

- `src/editor/panels/CameraPanel.tsx` - first under the panel's **More** disclosure since the usability pass (it was the third group, between size/position and movement). The ring is trim on a webcam whose shape, corner and size are already set above; `CameraPanel.md` carries the height arithmetic. Nothing about this component changed.
