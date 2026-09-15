# src/editor/shell/settings/InterfaceSection.tsx

`EditorSettingsDialog`'s "Interface" section - `ui.theme`, `ui.accent`, `ui.animated_brand` and `ui.ai_choreography`. `ui.accent` moved here from `BackgroundPanel` on 2026-09-15 (owner: an interface colour does not belong in a panel about the background), so every field of `ui` but the HUD-global `interface_effects` is this section's.

**Verified before building this (per the design spec's mandatory check): where does `ui.theme` actually persist and apply?** There are two independent `Settings` objects sharing the same Rust shape - the HUD's own global app config (`getSettings`/`saveSettings`, applied to `document.documentElement` via `applyTheme` in `Hud.tsx`, on mount and on every Preferences save) and `EditDoc.settings` (this doc's OWN per-project copy, written via `saveEdit`/`saveDocSettings`). `doc.settings.ui.theme` is the LATTER - and it is not a dead field: the Rust renderer reads it directly (`resolve_dark(settings.ui.theme)` in both `export/render/mod.rs` and `export/cursor/cursorpreview.rs`) to pick the dark/light synthetic cursor sprite baked into the export/preview. So it is a genuine per-project render setting, the same family as `ui.accent` (already doc-scoped, edited in `BackgroundPanel`) and `ui.animated_brand` (already doc-scoped, read by `Editor` for `brandState`) - writing it through `saveDocSettings` is the consistent path already established for its two siblings. No Rust change was needed: the field and both its consumers already existed.

## InterfaceSection

```tsx
export function InterfaceSection({ value, onChange }: {
  value: InterfaceSettings; onChange: (v: InterfaceSettings) => void;
}): JSX.Element
```

### Props

- `value: InterfaceSettings` - `doc.settings.ui` (the whole object, including `interface_effects`, the HUD's global knob this section does not edit - so its `set()` helper can spread `{ ...value, [k]: v }` without clobbering it).
- `onChange: (v: InterfaceSettings) => void` - called with the full next `InterfaceSettings`. `EditorSettingsDialog` composes this into `{ ...settings, ui }`.

### Controls

- **Theme** (`theme`, `Picker` - Light / Dark / System, mirrors the HUD's `SettingsInterface` `THEME_OPTS` labels exactly).
- **Accent** (`accent`, `Swatches` of the five `ACCENTS` presets from `panels/backgroundPresets.ts`), moved here from the Background panel on 2026-09-15.
- **Animated brand** (`animated_brand`, `Switch`, with the same "Flowing wave + REC pulse on the brand mark." hint the HUD's own `SettingsInterface` uses).
- **Replay applied edits with the pointer** (`ai_choreography`, `Switch`, M4 T5), hint "Off by default. The edits are already applied; this only performs them." The AI Director's fake pointer walks the applied edits after Apply (`director/useAiRun.md`). Doc-scoped like its two siblings: read off `doc.settings.ui` by `useAiRun` at apply time.

### Reset

The header's reset icon calls `onChange({ ...value, ...DEFAULT_INTERFACE_RESET })` - spreads `theme`/`accent`/`animated_brand`/`ai_choreography` over the CURRENT value, so `interface_effects` (the HUD's global knob) survives untouched.

## DEFAULT_INTERFACE_RESET

```ts
export const DEFAULT_INTERFACE_RESET: Pick<InterfaceSettings, "theme" | "accent" | "animated_brand" | "ai_choreography">
```

Mirrors `InterfaceSettings::default()` (`settings/model.rs`) for the four fields this section owns (`theme: "light"`, `accent: [239, 68, 68]`, `animated_brand: true`, `ai_choreography: false`).

### Used by

- `EditorSettingsDialog` (`src/editor/shell/settings/EditorSettingsDialog.tsx`) - `value={settings.ui}`, `onChange={setUi}`.
