# src/editor/controls/Slider.tsx

A custom range slider: pointer-drag track + spring thumb (Motion), and (as of Task 26) full
keyboard support per the WAI-ARIA slider pattern.

## snapToStep

```ts
export function snapToStep(raw: number, min: number, max: number, step: number): number
```

Snaps `raw` to the nearest `step`, clamped to `[min, max]`, with output decimal precision
matching `step`'s own (so e.g. `step=0.01` never produces `0.30000000000000004` from naive
floating-point arithmetic). Shared by the pointer-drag path (`updateValue`) and the keyboard path
(`sliderKeyValue` below) so both land on identical values for the same effective position.

## sliderKeyValue

```ts
export function sliderKeyValue(key: string, value: number, min: number, max: number, step: number): number | null
```

The value a key press should move the slider to, or `null` if `key` isn't one of the standard
slider keys. `ArrowRight`/`ArrowUp` and `ArrowLeft`/`ArrowDown` move by one `step`; `PageUp`/
`PageDown` move by 10 steps; `Home`/`End` jump to `min`/`max`. Every non-null result is already
passed through `snapToStep`.

## Slider

```tsx
export function Slider({ value, min, max, step, onChange, disabled, accentColor, ariaLabel }: {
  value: number; min: number; max: number; step?: number; onChange: (v: number) => void;
  disabled?: boolean; accentColor?: string; ariaLabel?: string;
}): JSX.Element
```

### Props

- `ariaLabel?: string` - (Task 26) sets `aria-label` on the `role="slider"` track. Every call
  site in the codebase passes this - a screen reader otherwise has no name for a bare slider.

### Behavior (Task 26 additions)

The track div is `role="slider"`, `tabIndex={disabled ? -1 : 0}`, with `aria-valuemin`/
`aria-valuemax`/`aria-valuenow` mirroring the numeric props, and an `onKeyDown` that calls
`sliderKeyValue` and, on a non-null result, `onChange`s straight to it. Focus is shown via
`.e-slider-track:focus-visible` (`editor.css`), a `--e-focus`-colored outline - the same token
used across the editor's other focus rings, not a new color.
