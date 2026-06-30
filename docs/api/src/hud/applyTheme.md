# src/hud/applyTheme.ts

Single utility that synchronizes the document's CSS theme attribute and accent color custom property with the current `ThemeMode` and accent color. Centralizes the dark/light resolution so every downstream component can rely on `data-theme` and `--accent` without re-reading settings.

## applyTheme

```ts
export function applyTheme(theme: ThemeMode, accent: [number, number, number]): void
```

Resolves the theme, writes `document.documentElement.dataset.theme`, and sets the `--accent` CSS custom property.

### Inputs

- `theme: ThemeMode` - one of `"light"`, `"dark"`, or `"system"`. *Why:* in `"system"` mode the function delegates to `matchMedia("(prefers-color-scheme: dark)")` at call time, so the OS controls the choice without requiring a separate code path.
- `accent: [number, number, number]` - RGB triplet, each value 0-255. *Why a tuple:* stored compactly in `InterfaceSettings`; the function formats it into a CSS `rgb()` string at call time rather than storing a formatted string.

### Returns

`void`. Side-effect only - mutates `document.documentElement`.

### Behavior

- Theme resolution: `"dark"` always writes `"dark"`; `"light"` always writes `"light"`; `"system"` reads `matchMedia("(prefers-color-scheme: dark)").matches` at the moment of the call and selects accordingly.
- There is no ongoing listener inside this function. The caller (`Hud.tsx`) must re-call `applyTheme` from a `MediaQueryList` `change` event to track OS preference switches at runtime.
- Sets `data-theme` on `<html>`, so CSS selectors like `[data-theme="dark"]` respond immediately with no flash.
- Sets `--accent` as an inline style on `<html>`, giving it higher specificity than any stylesheet variable definition.

### Used by

- `src/hud/Hud.tsx` - called on initial settings load, on user settings change, and from the `prefers-color-scheme` change listener
