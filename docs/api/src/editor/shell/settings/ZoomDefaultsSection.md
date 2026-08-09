# src/editor/shell/settings/ZoomDefaultsSection.tsx

`EditorSettingsDialog`'s "Zoom defaults" section - all 9 fields of `settings.zoom` (`ZoomSettings`), the seeds the auto-zoom generator (`export/camera/autozoom.rs`, via `ZoomSettings::to_zoom_config`) and the AI director's own zoom placement read as their starting point. Editing an already-placed zoom region goes through `ZoomInspector` instead; this section never touches `doc.zooms`.

## ZoomDefaultsSection

```tsx
export function ZoomDefaultsSection({ value, onChange }: {
  value: ZoomSettings; onChange: (v: ZoomSettings) => void;
}): JSX.Element
```

### Props

- `value: ZoomSettings` - `doc.settings.zoom`.
- `onChange: (v: ZoomSettings) => void` - called with the full next `ZoomSettings` on every control change (same full-object convention as `CursorPanel`/`EffectsPanel`'s `set()` helpers). `EditorSettingsDialog` composes this into `{ ...settings, zoom }` before calling `onSaveSettings`.

### Controls (spec order)

- **Zoom on click** (`enabled`, `Switch`).
- **Zoom amount** (`target_scale`, `Slider` 1.2-4, step 0.1, "x" suffix).
- **Hold / Idle release** (`hold_ms`, `NumberField` in seconds, 0.6-5s step 0.1) - label reads "Idle release" when `smart_hold` is on, "Hold" otherwise (mirrors the HUD's `SettingsZoom` label-swap exactly).
- **Smoothness** (`smoothness`, `Slider` 0.04-0.3, step 0.01) - the export's `follow_damping`.
- **Clicks to zoom** (`clicks`, 3-way `Picker` - "1"/"2"/"3" - via `clicksToOption`/`clicksFromOption` below).
- **Shrink camera on zoom** (`camera_shrink`, `Switch`).
- **Min camera size** (`camera_shrink_min`, `Slider` 0.3-1, step 0.02, "%" - only rendered while `camera_shrink` is on).
- **Smart type** (`smart_hold`, `Switch`).
- **Smart follow** (`smart_follow`, `Switch`).

All ranges/steps mirror the HUD's own `SettingsZoom` (`src/hud/settings/SettingsZoom.tsx`) exactly, for the same reason `EffectsPanel`'s `VMODES`/`TINTS` mirror the HUD's `SettingsClickFx` - one settings shape, one vocabulary, wherever it's edited.

### Reset

The header's reset icon (`.e-secrow`, `IconRotate2`) calls `onChange(DEFAULT_ZOOM_SETTINGS)` - replaces the WHOLE `ZoomSettings` object (all 9 exposed fields plus `cam_zoom_default`, not exposed here), matching the spec's "reset to `ZoomSettings::default()`" (not a partial patch of only this section's fields).

## clicksToOption

```ts
export const clicksToOption = (clicks: number): "1" | "2" | "3"
```

`clicks` is a Rust `u32` "clicks to trigger" count (`autozoom.rs`: a higher count "needs a quick multi-click rather than a single click"), reachable as 3 on real projects (seeded from the HUD's own `SettingsZoom` 1/2/3 picker at record time). A first version of this section used a `Switch` (1 vs 2), which silently collapsed a project's `clicks: 3` down to 1 or 2 the moment it was touched - a code-review Critical, since the value is genuinely reachable, not theoretical. This was replaced with a 3-way `Picker` mirroring the HUD's own `SettingsZoom.tsx` exactly (same three values, same bare "1"/"2"/"3" labels). `Picker<T extends string>` only takes string values, so `clicksToOption` is pure string encoding, not a lossy collapse: `clicks <= 1` -> `"1"`, `clicks === 2` -> `"2"`, anything else (i.e. `>= 3`) -> `"3"`.

## clicksFromOption

```ts
export const clicksFromOption = (opt: string): number
```

The inverse write: `Number(opt)` - "1"/"2"/"3" go straight back to `1`/`2`/`3`, never collapsed.

## DEFAULT_ZOOM_SETTINGS

```ts
export const DEFAULT_ZOOM_SETTINGS: ZoomSettings
```

Mirrors Rust `ZoomSettings::default()` (`settings/model.rs`) field-for-field, including `cam_zoom_default: null` - the one field this section has no control for, but Reset still has to land it on the real backend default since it replaces the whole object (the same bug class `CursorPanel`'s `DEFAULT_CURSOR_SETTINGS` guards against - see `CursorPanel.md`).

### Used by

- `EditorSettingsDialog` (`src/editor/shell/settings/EditorSettingsDialog.tsx`) - `value={settings.zoom}`, `onChange={setZoom}`.
