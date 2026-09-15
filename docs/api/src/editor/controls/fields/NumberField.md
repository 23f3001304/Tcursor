# src/editor/controls/fields/NumberField.tsx

A number input with +/- steppers - deliberately NOT a free-typed text field. The value only ever changes by a clamped `step` per click, so there is no empty/negative intermediate state a caller's `onChange` could ever observe mid-edit (unlike a raw `<input type="number">`, whose `onChange` fires on every keystroke, including a momentarily-cleared field where `Number("")` is `0`).

## NumberField

```tsx
export function NumberField({ value, min = 0, max, step = 0.1, unit = "s", onChange }: {
  value: number; min?: number; max?: number; step?: number; unit?: string; onChange: (v: number) => void;
}): JSX.Element
```

### Props

- `value: number` - the current value, shown verbatim (not itself clamped - callers are expected to only ever pass values already in range).
- `min?: number` - default `0`. The decrement button disables (`.e-numstep:disabled`) once `value <= min`; a decrement that would go below it is not applied.
- `max?: number` - no ceiling when omitted. The increment button disables once `value >= max`; an increment that would exceed it is not applied.
- `step?: number` - default `0.1`. Each click moves `value` by exactly this much, rounded to 2 decimals (`+(value ± step).toFixed(2)`).
- `unit?: string` - default `"s"` (seconds - the original hardcoded suffix, still the default so every existing caller, e.g. `ZoomInspector`'s Start/End/Zoom-in/Zoom-out, is unaffected). Pass `""` to suppress the suffix entirely (e.g. `CameraMoveInspector`'s X/Y/Size, which are 0-1 fractions, not a time value).
- `onChange: (v: number) => void` - called with the new value on a successful increment/decrement (never called when the step would cross `min`/`max`).

### Behavior

Renders a `.e-numfield` pill (design/premium-pass D4: `--e-raised` background, hairline border, `--e-inset-hi` top-light, `--e-line-strong` on hover - the border doctrine's full raised-control treatment): a decrement `IconMinus` button, a centered read-only value display (`{value}{unit}`, tabular-nums), and an increment `IconPlus` button. Both buttons use the shared `.e-numstep` look (`editor.css`) for hover/active/disabled states.

### Used by

`ZoomInspector` (Start/End/Zoom-in/Zoom-out, all seconds - default `unit`) and `CameraMoveInspector` (Time in seconds; X/Y/Size as bare 0-1 fractions via `unit=""`).
