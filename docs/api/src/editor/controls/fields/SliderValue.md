# src/editor/controls/fields/SliderValue.tsx

A slider's label and its live value on one row, where the value can be typed. Added by the panel pass (spec `2026-09-13-panel-design-benchmark.md`, section (b) point 14: "numeric value inline with the control, editable by direct-drag or typing").

`Slider` renders this whenever it is given a `label`; no call site talks to it directly, and `Slider`'s props did not change.

**Why typing matters at all.** A 320px panel gives a track about 290px. Corner Radius runs 0 to 80px and Gradient Angle 0 to 360 degrees, so one pixel of track is a third of a degree at best: dragging cannot reliably reach an exact number that a user already knows they want. Arrow keys could, but only after tabbing to the track and counting presses.

## parseSliderInput

```ts
export function parseSliderInput(text: string, min: number, max: number, step: number): number | null
```

The number a typed readout should commit to, or `null` when the text holds no number at all - which the caller treats as "leave the value alone", never as 0.

The field opens pre-filled with the bare number, but the regex tolerates whatever unit the formatter printed around it (`"35 %"`, `"1.20x"`, `"160 deg"`, `"-40 ms"`), because a user who sees `35%` and retypes the whole thing should not be told they are wrong. The result is run through `snapToStep`, so a typed value can never land off-step or out of range - the same guarantee the drag and keyboard paths already had.

### Behaviors

- `reads a bare number, snapped and clamped to the slider's own range`.
- `tolerates the unit the formatter printed, so retyping it is not an error`.
- `returns null for text holding no number at all, so the caller writes nothing`.

## SliderValue

```tsx
export function SliderValue({ label, value, text, min, max, step, disabled, onCommit }: {
  label: string; value: number; text: string;
  min: number; max: number; step: number;
  disabled?: boolean; onCommit: (v: number) => void;
}): JSX.Element
```

### Props

- `value: number` - the LIVE number (`Slider`'s `shown`, i.e. the drag override when there is one), used as the edit field's starting text and as the "did this actually change" comparison on commit.
- `text: string` - that same number already run through the caller's `formatValue`. Kept separate from `value` so the readout displays units while the edit field opens on the bare number.
- `onCommit` - `Slider`'s `commitNow`, which cancels any pending drag debounce and calls `onChange` immediately. A typed value is discrete, so there is nothing to coalesce.

### Behavior

The value is a `<button>` until clicked, then an uncontrolled `<input>` with the same box, so committing does not shift the row. **Enter** commits and closes. **Esc** reverts: a `reverting` ref is set before the state change, and the `onBlur` that follows sees it and commits nothing. **Blur** commits - clicking away from a number you just typed is not a silent discard.

The readout tracks the drag because `Slider` passes `shown`, not the committed `value` prop - see `Slider.md`'s "Optimistic local value".

### Markup note

Both boxes are 24px tall (19px until the usability pass put a floor under every clickable control): the readout is a real button that opens a text field, and at 19px it was the smallest click target in the editor. The row grew 5px with it, which is the one place the pass spent height rather than saving it.

The row is `.e-fl` (the existing label class) holding `.e-fl-name` and either `.e-val` or `.e-val-edit`, with a literal space between them. That space is what the previous one-string markup (`{label} <b>{value}</b>`) produced, and callers' tests read the row as text (`"Factor 2x"`); a whitespace-only text node is never rendered as a flex item, so it costs nothing in layout.

### Used by

- `src/editor/controls/fields/Slider.tsx` - rendered in place of the old static `.e-fl` readout.
