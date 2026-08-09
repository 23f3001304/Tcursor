# src/editor/shell/settings/InterfaceSection.tsx

`EditorSettingsDialog`'s "Interface" section - `ui.theme` and `ui.animated_brand`. `ui.accent` lives on the same `InterfaceSettings` struct but stays `BackgroundPanel`'s own control (Accent Colors swatches); this section deliberately never touches it, including on Reset.

**Verified before building this (per the design spec's mandatory check): where does `ui.theme` actually persist and apply?** There are two independent `Settings` objects sharing the same Rust shape - the HUD's own global app config (`getSettings`/`saveSettings`, applied to `document.documentElement` via `applyTheme` in `Hud.tsx`, on mount and on every Preferences save) and `EditDoc.settings` (this doc's OWN per-project copy, written via `saveEdit`/`saveDocSettings`). `doc.settings.ui.theme` is the LATTER - and it is not a dead field: the Rust renderer reads it directly (`resolve_dark(settings.ui.theme)` in both `export/render/mod.rs` and `export/cursor/cursorpreview.rs`) to pick the dark/light synthetic cursor sprite baked into the export/preview. So it is a genuine per-project render setting, the same family as `ui.accent` (already doc-scoped, edited in `BackgroundPanel`) and `ui.animated_brand` (already doc-scoped, read by `Editor` for `brandState`) - writing it through `saveDocSettings` is the consistent path already established for its two siblings. No Rust change was needed: the field and both its consumers already existed.

## InterfaceSection

```tsx
export function InterfaceSection({ value, onChange }: {
  value: InterfaceSettings; onChange: (v: InterfaceSettings) => void;
}): JSX.Element
```

### Props

- `value: InterfaceSettings` - `doc.settings.ui` (the whole object, including `accent`, even though this section only edits two of its three fields - so its `set()` helper can spread `{ ...value, [k]: v }` without clobbering `accent`).
- `onChange: (v: InterfaceSettings) => void` - called with the full next `InterfaceSettings`. `EditorSettingsDialog` composes this into `{ ...settings, ui }`.

### Controls

- **Theme** (`theme`, `Picker` - Light / Dark / System, mirrors the HUD's `SettingsInterface` `THEME_OPTS` labels exactly).
- **Animated brand** (`animated_brand`, `Switch`, with the same "Flowing wave + REC pulse on the brand mark." hint the HUD's own `SettingsInterface` uses).

### Reset

The header's reset icon calls `onChange({ ...value, ...DEFAULT_INTERFACE_RESET })` - spreads only `theme`/`animated_brand` over the CURRENT value, so `accent` survives untouched (never part of this section's reset target).

## DEFAULT_INTERFACE_RESET

```ts
export const DEFAULT_INTERFACE_RESET: Pick<InterfaceSettings, "theme" | "animated_brand">
```

Mirrors `InterfaceSettings::default()` (`settings/model.rs`) for exactly the two fields this section owns (`theme: "light"`, `animated_brand: true`) - `accent`'s own Rust default (`[239, 68, 68]`) is `BackgroundPanel`'s reset to assert, not this section's.

### Used by

- `EditorSettingsDialog` (`src/editor/shell/settings/EditorSettingsDialog.tsx`) - `value={settings.ui}`, `onChange={setUi}`.
