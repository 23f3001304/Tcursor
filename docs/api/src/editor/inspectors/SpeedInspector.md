# src/editor/inspectors/SpeedInspector.tsx

The selected speed span's properties: how fast it runs, and the two edges it runs between. Routed to by `PropertiesSlot` whenever `sel` matches a `doc.speed` entry.

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Timing**, **Rate**, then Remove - the shared reading order from `InspectorShape.md`. Timing comes first here even though Factor is the more interesting control, because every inspector answers "when is this" before anything else; the header lede and the Timing fields are the same span, in clock time and in seconds.

## FACTOR_SNAPS

```ts
export const FACTOR_SNAPS = [0.5, 1, 1.5, 2, 4, 8];
```

The factors worth landing on exactly, from the spec. A slider fine enough to reach 1.85 is also fine enough to MISS 2, which is the value a user actually meant nine times out of ten.

## snapFactor

```ts
export function snapFactor(v: number): number
```

The nearest `FACTOR_SNAPS` entry within `SNAP_WINDOW` (0.08), else the raw value rounded to two decimals. Applied on change, not on release, so the readout shows the snapped value while the thumb moves rather than jumping after it. Everything between the snaps stays reachable - this narrows the target, it does not quantize the slider.

## SpeedInspector

```tsx
export function SpeedInspector({ speed, dur, onApply, onClose }: {
  speed: Speed; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `speed: Speed` - the selected span, found by id in `doc.speed`.
- `dur: number` - the clip's duration, the End field's ceiling.
- `onApply` - every control applies one `update_speed` (or `remove_speed`); the preview retimes as the slider moves, the same live-edit contract `ZoomInspector` has.
- `onClose` - drops the selection.

### Behavior

**Factor.** `Slider` from `FACTOR_MIN` to `FACTOR_MAX` (0.25 to 8, imported from `lib/remap.ts` so the UI bound and the clock map's clamp are literally the same constants), `step` 0.05, `snapFactor` on change, formatted `2x` / `0.5x`, accented `--e-speed` - the same accent the span's own pill carries on the Time lane, so the two read as one object.

**The hint does the arithmetic.** "2 s of recording becomes 1 s of export." - the span's clip length over its factor, which is the number a user is actually deciding about. It sits under the Rate slider, next to the control that changes it.

**Remove.** `remove_speed` then `onClose`; the same op Delete on a selected span applies (`useEditorKeymap`).

### Notes

- `Slider` debounces its own commits and flushes on release (`Slider.tsx`), so a drag across the range is a handful of ops, not one per pixel.
- The Rust ops normalise after every edit: the factor is clamped into `[0.25, 8]` and a span is clamped to start at its predecessor's end, so extending one span's End past the next one's Start pushes that neighbour rather than overlapping it. The inspector reads the span back out of the doc every render for that reason.
