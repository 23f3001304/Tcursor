# src/editor/controls/fields/ColorInput.tsx

A native `<input type="color">` dressed as one of the panel's swatches - the custom half of every "pick a preset OR pick your own" control (`BackgroundPanel`'s Color tab, `GradientTab`'s stops). The OS owns the picker popover, so this component owns only the trigger's look and the RGB <-> hex conversion.

*Why the native input and not a hand-built picker:* a colour picker is a surprisingly deep control (hue ring, saturation field, eyedropper, recent colours, accessibility). The platform already ships one that WebView2 renders properly, and the editor's swatch language survives around it with about ten lines of CSS. No new dependency either, which the same reasoning rules in.

## toHex

```ts
export function toHex(c: [number, number, number]): string
```

`[r, g, b]` -> `#rrggbb`, the only form `<input type="color">` accepts.

Each channel is clamped to `0..255` and rounded before being zero-padded to two hex digits. *Why clamp here rather than trust the caller:* the input element silently IGNORES a malformed `value`, which would strand the swatch showing a stale colour with no error anywhere - so the conversion has to produce six valid digits for any number it is handed.

## fromHex

```ts
export function fromHex(hex: string): [number, number, number]
```

`#rrggbb` -> `[r, g, b]`. Leading/trailing whitespace is trimmed and the digits are case-insensitive. Anything else (empty, a colour name, the three-digit shorthand) returns black rather than throwing: the input element only ever emits the canonical form, so this is a type guard at the boundary, not a general CSS colour parser.

### Behaviors

- `round-trips every channel, zero-padding single-digit components` - `toHex`/`fromHex` are exact inverses over the whole range.
- `clamps and rounds out-of-range channels instead of emitting an invalid hex` - `[-5, 300, 12.6]` becomes `#00ff0d`.
- `treats anything that is not a #rrggbb string as black rather than throwing`.

## ColorInput

```tsx
export function ColorInput({ value, onChange, ariaLabel, label }: {
  value: [number, number, number];
  onChange: (c: [number, number, number]) => void;
  ariaLabel: string;
  label?: string;
}): JSX.Element
```

The swatch itself: a `.e-colorpick` label wrapping the input, with an optional caption underneath (`GradientTab` labels its From / Middle / To stops).

- `value` - the current colour, in the same `[r, g, b]` shape settings store.
- `onChange` - fired on every input event, so dragging in the OS picker updates live. Callers debounce downstream if their write is expensive (`Slider`'s own commit debounce covers the sliders next to it).
- `ariaLabel` - required: the swatch has no visible text of its own unless `label` is passed.

### Used by

- `src/editor/panels/background/BackgroundPanel.tsx` - the Color tab's custom colour.
- `src/editor/panels/background/GradientTab.tsx` - the two or three gradient stops.
