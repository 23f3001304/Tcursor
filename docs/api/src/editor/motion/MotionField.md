# src/editor/motion/MotionField.tsx

The Motion section's body, shared by the zoom, layout and camera-move inspectors (M3): the preset row (`PresetRow.md`), the graph of the region's whole motion under it (`MotionGraph.md`), and, only when the curve is a spring, the two spring sliders (`inspectors/SpringControls.md`). One component so the three inspectors cannot drift apart in how a preset lands: a pick writes BOTH ramps' curves through `onPatch`, exactly as a key drag does, and the inspector maps the patch onto its own op's field names (`zoom_in_ms` / `transition_ms`, and so on).

**Why the spring sliders stay.** A spring has no keys to drag: the graph draws its sampled oscillator (`editable: false` from `buildGraph`), and stiffness and damping are its only two numbers, so `SpringControls` remains their editor. A spring picked from the row (Bouncy) writes the same string onto both ramps; a slider move does the same.

## MotionField

```tsx
export function MotionField({ input, easing, easingOut, retimeable = true, onPatch }: {
  input: GraphInput;
  easing: string;
  easingOut?: string | null;
  retimeable?: boolean;
  onPatch: (patch: GraphPatch) => void;
}): JSX.Element
```

### Props

- `input: GraphInput` - the region as the graph draws it, built by the inspector (`zoomGraphInput`, `layoutGraphInput`, `camGraphInput`).
- `easing: string` - the in ramp's curve, the string the preset row is matched against.
- `easingOut?: string | null` - the out ramp's curve. `null` / `undefined` means "the same curve as `easing`": a zoom with no `easing_out`, or a camera move, which has one ramp.
- `retimeable?: boolean` - forwarded to the graph; a camera move passes `false`, since its ramp length is the keyframe spacing.
- `onPatch: (patch: GraphPatch) => void` - receives `{ easing?, easing_out?, inMs?, outMs? }` from a preset pick, a key drag or a spring slider; the inspector turns it into its update op.

## motionReadout

```ts
export function motionReadout(easing: string, easingOut?: string | null): string | undefined
```

The heading-row readout for a Motion section: `"Custom"` when the curve pair matches no preset (`presetOf`), else `undefined`, since with a preset matched the lit segment already names it and repeating it on the heading would be the same word twice.

### Used by

- `src/editor/inspectors/ZoomInspector.tsx`, `LayoutInspector.tsx`, `CameraMoveInspector.tsx` - each inspector's Motion section.
