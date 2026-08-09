# src/hud/settings/SettingsInterface.tsx

Settings panel for app-level interface preferences: theme mode, accent color, and (Task 39) the animated-brand feel knob. Also renders a stub Language field that is disabled pending i18n support.

## SettingsInterface

```tsx
export function SettingsInterface({
  value,
  onChange,
}: {
  value: InterfaceSettings;
  onChange: (v: InterfaceSettings) => void;
})
```

Edits the `ui` group of `Settings`.

### Props

- `value: InterfaceSettings` - current interface settings (`theme`, `accent`, and `animated_brand`). *Why:* controlled component; the parent (Preferences) owns and persists state.
- `onChange: (v: InterfaceSettings) => void` - receives a fully replaced `InterfaceSettings`. *Why:* immutable replacement keeps the parent's persistence path uniform across all panels.

### Behavior

Renders `<section className="sec">` with heading "Interface". Four `Field` sections in order:

1. **App theme** - `.seg`/`.seg-btn` segmented control. Three options from `THEME_OPTS`:
   - `"light"` - Light
   - `"dark"` - Dark
   - `"system"` - System (follows OS preference)
   
   Writes `value.theme` (`ThemeMode`). Selection state is determined by `value.theme === t.id`. *Why three options:* "System" allows users who work in both environments to avoid manually switching; the explicit Light/Dark overrides are needed when a user wants the app to differ from OS preference.

2. **Accent** - `.swatches` button row. Five preset RGB tuples from `ACCENTS`:
   - `[239, 68, 68]` - red
   - `[91, 91, 214]` - indigo
   - `[47, 107, 255]` - blue
   - `[16, 185, 129]` - green
   - `[245, 158, 11]` - amber

   Writes `value.accent` (`[number, number, number]`). Selection is compared via `rgb()` string equality. *Why string comparison:* RGB arrays are value types but JS compares arrays by reference, so string conversion is the simplest correct equality check.

3. **Animated brand** (Task 39) - a `Switch` bound to `value.animated_brand`, hint text "Flowing wave + REC pulse". Writes `value.animated_brand` (`boolean`). *Why here:* the fake-polish rule is every feel knob is a setting; this is the one UI control for `InterfaceSettings.animated_brand` (see `settings.md`), which gates whether `TcursorMark` flows/pulses at all in both the HUD titlebar and the editor's `TopBar`.

4. **Language** - a single `.seg-btn` with label "English" that is `disabled` and has `cursor: default`. No settings field is written. *Why rendered:* it signals that i18n is planned without requiring partially implemented locale switching.

### Notes

- `rgb` is a file-local helper: `` (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})` ``. It is used both to generate the inline `background` style for swatch buttons and to compare the current accent against each preset.
- `THEME_OPTS` and `ACCENTS` are file-local constants; neither is exported. Changing the available presets requires editing this file directly.
