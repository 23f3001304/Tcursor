# src/editor/controls/Swatches.tsx

The one shared component (Task 26) behind every "pick a preset" swatch grid/row in the editor:
`BackgroundPanel`'s color/gradient/accent grids, `EffectsPanel`'s ripple-color/tint rows,
`CameraRingField`'s ring-color row. Previously each of these hand-rolled its own near-identical
button markup (some using the `.e-preset-circle`/`.e-accent-circle` CSS classes with a box-shadow
selection ring, others inline-styled 22px circles with a border-based selection) - `Swatches`
renders both looks (see `variant` below) from one place, but changes NO visuals: each existing
look is now a real CSS class instead of scattered inline `style={{...}}` objects.

## SwatchVariant

```ts
export type SwatchVariant = "preset" | "accent" | "small"
```

Which existing look to render, unchanged pixel-for-pixel from before this component existed:
- `"preset"` - `.e-preset-grid`/`.e-preset-circle` (BackgroundPanel's color/gradient grids - a
  6-column grid, box-shadow selection ring).
- `"accent"` - `.e-accent-list`/`.e-accent-circle` (BackgroundPanel's accent row - a flex row of
  fixed 24px circles, same box-shadow ring).
- `"small"` (default) - `.e-swatch-row`/`.e-swatch-sm` (EffectsPanel's ripple-color/tint,
  CameraRingField's ring-color - a flex row of 22px circles, border-based selection: 2px `--e-fg`
  when selected vs 1px `--e-border` otherwise).

## SwatchItem

```ts
export interface SwatchItem<T> { key: string; css: string; value: T; ariaLabel?: string }
```

One swatch. `css` is what actually paints the button's `background` - a solid `rgb(r, g, b)` or a
`linear-gradient(...)` string (the caller builds this; `Swatches` never interprets color data
itself, which is what lets it serve both plain-RGB presets and BackgroundPanel's 2-stop gradient
presets identically). `value` is the caller's own data for that swatch (e.g. the RGB tuple, or a
`GradientPreset` object) - handed back to `isSelected`/`onSelect` unchanged, so callers never have
to re-derive it from `css`. `ariaLabel` defaults to `key` when omitted.

## Swatches

```tsx
export function Swatches<T>({ items, isSelected, onSelect, variant, disabled }: {
  items: SwatchItem<T>[];
  isSelected: (value: T) => boolean;
  onSelect: (value: T) => void;
  variant?: SwatchVariant;
  disabled?: boolean;
}): JSX.Element
```

Renders `items` as a row/grid of buttons (per `variant`); each button's `aria-label`, `on` class
(from `isSelected`), and `onClick` (calling `onSelect` with that item's own `value`) are wired
per-swatch. `disabled` (only meaningful in practice for `"small"`, e.g. EffectsPanel's Ripple
Color when the ripple style is "None") disables every button and applies the standard dimmed
look (`.e-swatch-sm:disabled`).

### Used by

- `src/editor/panels/BackgroundPanel.tsx` - color presets, gradient presets (`variant="preset"`), accent colors (`variant="accent"`)
- `src/editor/panels/EffectsPanel.tsx` - ripple color, spotlight tint (`variant="small"`, the default)
- `src/editor/panels/CameraRingField.tsx` - ring color (`variant="small"`)
