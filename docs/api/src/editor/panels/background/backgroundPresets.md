# src/editor/panels/background/backgroundPresets.ts

The Background panel's static tables: the solid-colour swatches, the accent swatches, and the doc-level default every Reset lands on. Deliberately plain data in the exact shape the settings store, so a swatch can never render as something the backend cannot reproduce. The GRADIENT presets are NOT here - they moved to Rust (`settings::wallpapers::GRADIENT_WALLPAPERS`) when they gained thumbnails and an optional middle stop, and the panel reads them off `background_thumbs` so there is one table, not two that drift.

## DEFAULT_BG

```ts
export const DEFAULT_BG: BackgroundSettings
```

`BackgroundSettings::default()` on the Rust side, field for field: `kind: "mesh"` with an empty `mesh` (the legacy Classic `bg.jpg`, which is what every pre-library project already renders), the default solid and gradient stops, `blur: 0`, `asset: null`, `dim: 0`.

It lives here rather than in `BackgroundPanel.tsx` because the panel is at its line budget and because this is data, not UI. *Why `asset: null` is safe in a Reset:* clearing the field clears the CHOICE, not the file. The imported asset stays in `<project>/background/`, so Reset is undoable by re-selecting the asset card; only the card's own Remove deletes anything.

### Used by

- `src/editor/panels/background/BackgroundPanel.tsx` (`handleReset`) - the one consumer.
- `src-tauri/src/settings/background.rs` (`BackgroundSettings::default`) - the value this mirrors. A drift between the two is exactly the bug this file exists to prevent (`CursorPanel`'s own default had one).

## NamedColor

```ts
export interface NamedColor { rgb: [number, number, number]; name: string }
```

One swatch: the plain RGB triple `settings.background.solid` / `settings.ui.accent` store, plus the display name used as the swatch's aria-label.

## COLOR_PRESETS

```ts
export const COLOR_PRESETS: NamedColor[]
```

Twelve dark, recording-friendly background colours for the panel's Color tab. Applied as `{ kind: "solid", solid: rgb }`.

## ACCENTS

```ts
export const ACCENTS: NamedColor[]
```

Five UI accent colours (`settings.ui.accent`). Unrelated to the background itself; they share this file because they share the swatch component and the panel. Since 2026-09-15 the swatches are drawn by `shell/settings/InterfaceSection.tsx`, not by this panel.

## rgb

```ts
export const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`
```

An RGB triple as a CSS colour string. Used both to paint a swatch and, because the result is a canonical string, to compare the current background against a preset without a triple-wise equality helper: `rgb(bg.solid) === rgb(c)` is the panel's selection test.

It moved here out of `BackgroundPanel.tsx` when the Color section landed, because that file was at 197 of its 200-line budget and this is data plumbing rather than UI. `COLOR_ITEMS` came with it, since it is the only other caller.

## COLOR_ITEMS

```ts
export const COLOR_ITEMS: SwatchItem<[number, number, number]>[]
```

`COLOR_PRESETS` in the shape the `Swatches` control takes: the CSS string as both the React key and the painted colour, the raw triple as the value handed back on select, and the preset's display name as the aria-label. Built once at module load rather than per render, because the table is static.

### Used by

- `src/editor/panels/background/BackgroundPanel.tsx` - the Color tab's preset swatches.
