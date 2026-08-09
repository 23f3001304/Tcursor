# src/editor/shell/settings/ScreenSection.tsx

`EditorSettingsDialog`'s "Screen" section - the one `appearance.screen` field with no other editor surface: `screen_size`. The other four appearance fields on this same `ModeAppearance` (`pad`, `screen_radius`) are already `BackgroundPanel`'s controls (frame padding/corner radius); the other four layout modes' own appearance (`screen_only`/`camera`/`camera_only`/`presenter`) are out of scope for this task (T34 territory).

## ScreenSection

```tsx
export function ScreenSection({ value, onChange }: {
  value: ModeAppearance; onChange: (v: ModeAppearance) => void;
}): JSX.Element
```

### Props

- `value: ModeAppearance` - `doc.settings.appearance.screen` (the Screen layout mode's appearance block).
- `onChange: (v: ModeAppearance) => void` - called with the full next `ModeAppearance` (only `screen_size` ever actually changes here). `EditorSettingsDialog` composes this into `{ ...settings, appearance: { ...settings.appearance, screen } }`.

### Controls

- **Screen size** (`screen_size`, `Slider` 0.6-1, step 0.01, "%") - scales the screen panel toward its own center; the range mirrors the Rust field comment (`settings/appearance.rs`: "0.6..1.0 scale of the screen panel about its center") exactly.

### Reset

The header's reset icon calls `onChange({ ...value, screen_size: DEFAULT_SCREEN_SIZE })` - touches `screen_size` only, leaving `pad`/`screen_radius` (BackgroundPanel's fields, on this same object) untouched, matching the spec's "Screen resets screen_size only".

## DEFAULT_SCREEN_SIZE

```ts
export const DEFAULT_SCREEN_SIZE = 1.0
```

Mirrors `ModeAppearance::default().screen_size` (`settings/appearance.rs`) exactly - the screen panel at full size, today's unedited render.

### Used by

- `EditorSettingsDialog` (`src/editor/shell/settings/EditorSettingsDialog.tsx`) - `value={settings.appearance.screen}`, `onChange={setScreen}`.
