# src/editor/panels/backgroundPresets.ts

The Background panel's static tables: the solid-colour swatches, the accent swatches, and the doc-level default every Reset lands on. Deliberately plain data in the exact shape the settings store, so a swatch can never render as something the backend cannot reproduce. The GRADIENT presets are NOT here - they moved to Rust (`settings::wallpapers::GRADIENT_WALLPAPERS`) when they gained thumbnails and an optional middle stop, and the panel reads them off `background_thumbs` so there is one table, not two that drift.

## DEFAULT_BG

```ts
export const DEFAULT_BG: BackgroundSettings
```

`BackgroundSettings::default()` on the Rust side, field for field: `kind: "mesh"` with an empty `mesh` (the legacy Classic `bg.jpg`, which is what every pre-library project already renders), the default solid and gradient stops, `blur: 0`, `asset: null`, `dim: 0`.

It lives here rather than in `BackgroundPanel.tsx` because the panel is at its line budget and because this is data, not UI. *Why `asset: null` is safe in a Reset:* clearing the field clears the CHOICE, not the file. The imported asset stays in `<project>/background/`, so Reset is undoable by re-selecting the asset card; only the card's own Remove deletes anything.

### Used by

- `src/editor/panels/BackgroundPanel.tsx` (`handleReset`) - the one consumer.
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

Five UI accent colours (`settings.ui.accent`). Unrelated to the background itself; they share this file because they share the swatch component and the panel.
