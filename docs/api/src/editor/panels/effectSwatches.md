# src/editor/panels/effectSwatches.ts

The Effects panel's two colour palettes and the one adapter that turns either of them into `Swatches` items. Split out of `EffectsPanel.tsx` when the spotlight half became its own file: both halves paint a swatch row, and a palette that lived in one of them would have had to be imported from the other.

## NamedColor

```ts
export type NamedColor = [[number, number, number], string];
```

A colour and the word for it. The name is not decoration: it becomes the swatch's `aria-label`, so a screen reader says "Violet" rather than "rgb(130, 90, 255)". The HUD's own swatches use the bare `rgb(...)` string as their label, so the names here are local to the editor.

## RIPPLE_COLORS

```ts
export const RIPPLE_COLORS: NamedColor[]
```

White, Red, Blue, Green, Orange - the click-ripple palette, written to `settings.color`. Kept in lockstep with `src/hud/settings/SettingsClickFx.tsx`'s `SWATCHES` by convention, not by a shared import: the HUD ships without the editor's controls and neither side should pull the other in for five tuples.

## SPOTLIGHT_TINTS

```ts
export const SPOTLIGHT_TINTS: NamedColor[]
```

Violet, Blue, Green, Red, Yellow - the always-on spotlight's tint, written to `settings.spotlight_tint`. The colour VALUES mirror the HUD's `TINTS` exactly.

## rgb

```ts
export const rgb: (c: [number, number, number]) => string
```

`[r, g, b]` to a CSS `rgb(r, g, b)` string. Also the equality test for "which swatch is selected": comparing the two strings is cheaper than comparing three numbers and reads the same at both call sites.

## swatchItems

```ts
export const swatchItems: (colors: NamedColor[]) => SwatchItem<[number, number, number]>[]
```

Adapts a palette to the `Swatches` control's item shape (`{key, css, value, ariaLabel}` - see `Swatches.md`). The `key` and the `css` are both the `rgb(...)` string, the `value` is the tuple the setting stores, and the `ariaLabel` is the palette's own name for it.

### Used by

- `src/editor/panels/EffectsPanel.tsx` - Ripple Color, over `RIPPLE_COLORS`.
- `src/editor/panels/SpotlightSection.tsx` - Tint, over `SPOTLIGHT_TINTS`.
