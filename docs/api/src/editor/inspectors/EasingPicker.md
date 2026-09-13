# src/editor/inspectors/EasingPicker.tsx

The segmented row of named curves that sits above `CurveEditor`'s canvas. It is not used on its own:
`CurveEditor` is the block callers reach for, and this is the half of it that names curves.

It has been three things. A grid of six curve cards (vetoed: cards in a panel). A `Picker` dropdown
(which lost the ability to shape a curve, and the owner asked for that back). Now a `SegRow` of
planes with the gliding accent tick every other exclusive choice in the inspectors uses, so the
curve row, a zoom's Feel row and a layout's preset row are visibly one control family.

Keys and names come from `src/editor/timeline/curveGlyphs.ts`, the one list the timeline's transition
popover and the export's `ease` also agree with; adding a curve there shows it here, though a new key
still needs an `ease` branch and a `valid_easing` arm on the Rust side.

## easingOption

```ts
export function easingOption(value: string): string
```

The option a stored easing string selects: any spring (the bare word `"spring"` or a parameterised
`spring(...)`) is the **Spring** option, a `cubic(...)` is **Custom**, a known key is itself, and
anything unrecognised falls back to `smooth` - what `valid_easing` coerces it to anyway.

Without the spring branch, tuning the stiffness slider would deselect the very segment doing the
tuning, since `spring(300,10)` is not a key match.

### Used by

- `EasingPicker` - drives which segment is `on`.

## EasingPicker

```tsx
export function EasingPicker({ value, onChange, label = "Transition curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}): JSX.Element
```

A `.e-fl` label (the setting's own name, "Transition curve" or "Exit Curve") over a `SegRow` of the
six named curves. A seventh **Custom** segment appears only while the value is a cubic - what
dragging a handle on the canvas writes. Picking Custom is a no-op: there is nothing to switch back
*to*, since the handles are how you leave it, so it renders selected-and-inert rather than as a
choice. Picking the already-selected segment is also a no-op, so a stray click never writes an op.

`label` is also what the two rows on a layout segment differ by, so "Transition curve" and "Exit
Curve" read as two settings rather than one repeated twice. Their gliding ticks are kept apart by
`SegRow`'s own per-instance `useId()`, not by the label.
