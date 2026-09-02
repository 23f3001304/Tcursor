# src/hud/settings/SettingsClickFx.tsx

Settings panel for click effects, cursor spotlight, and video effects. Exposes three independently controlled sub-groups - click animations on mouse presses, a persistent spotlight around the cursor, and a full-frame video color effect toggled via hotkey.

## SettingsClickFx

```tsx
export function SettingsClickFx({
  value,
  onChange,
}: {
  value: ClickFxSettings;
  onChange: (v: ClickFxSettings) => void;
})
```

Edits the `clickfx` group of `Settings`.

### Props

- `value: ClickFxSettings` - current state of all effect fields. *Why:* controlled component; the parent (Preferences) owns and persists state.
- `onChange: (v: ClickFxSettings) => void` - receives a fully replaced `ClickFxSettings`. *Why:* immutable replacement simplifies the parent's save logic; no partial-update diffing is needed.

### Behavior

Renders `<section className="sec">` with no top-level heading (removed - it sat directly above the first inline group's own "Click effects" label with nothing between them, reading as a duplicated, empty header; the "FX" settings tab pill is title enough). Three inline uppercase-label headers divide the controls:

**Click effects**

- `enabled` (Switch, "Show click effects") - master gate. When false, the style, color, and intensity controls are hidden. *Why conditional:* no reason to expose style or color pickers when the feature is off.
- `style` (Picker / `.optg`, visible only when `enabled`) - selects `ClickFxStyle`: none / ripple / pulse / glow / shockwave / particles / neon. Writes `value.style`.
- `color` (Swatches, visible only when `enabled`) - five preset RGB tuples from `SWATCHES` (white, red, blue, green, yellow). Writes `value.color`.
- `intensity` (Range 0.2-1, step 0.05, visible only when `enabled`) - writes `value.intensity`, displayed as a percentage. *Why minimum 0.2:* zero would render invisibly, making the feature appear broken.
- `captions` (Switch, "Keystroke captions") - always visible regardless of `enabled`. Writes `value.captions`. *Why independent of enabled:* captions are a distinct overlay type not tied to mouse-click animations.

**Cursor spotlight**

- `spotlight` (Switch, "Always on") - writes `value.spotlight`. When true, the spotlight renders continuously; when false it is only triggered by the hotkey.
- `spotlight_mode` (Picker with hint "also used by the hotkey") - selects `SpotlightMode`: classic / blur / halo / breathing / nebula / vignette. Writes `value.spotlight_mode`. *Why the hint:* the same mode enum drives both the always-on overlay and the hotkey-triggered version, so the user sets style in one place.
- `spotlight_tint` (Swatches with `TINTS`) - five preset RGB tuples (purple, blue, green, red, yellow). Writes `value.spotlight_tint`. Uses a separate palette from click-effect `SWATCHES`.
- Advanced (collapsed by default):
  - `spotlight_dim` (Range 0.2-0.9, step 0.05) - background darkening. Writes `value.spotlight_dim`.
  - `spotlight_radius` (Range 0.05-0.30, step 0.01) - lit-area radius as a fraction of frame width. Writes `value.spotlight_radius`.
  - `spotlight_feather` (Range 0.02-0.25, step 0.01) - soft-edge width. Writes `value.spotlight_feather`.

**Video effect**

- `video_fx_mode` (Picker with hint "hold hotkey to activate") - selects `VideoFxMode`: nebulawash / cinematicdim / screenfocus / colorpop. Writes `value.video_fx_mode`. No enable toggle here; activation is always hotkey-driven.

### Notes

- `Picker` and `Swatches` are file-private helpers. `Picker` wraps the `.optg`/`.opt` pill pattern. `Swatches` compares selection by `rgb()` string equality, not by array reference.
- The typed `set` shorthand always produces `onChange({ ...value, [k]: v })` - a fresh top-level object - so all other fields are preserved unchanged.
- `SWATCHES` (click color) and `TINTS` (spotlight tint) are different constant arrays with different default palettes and are not interchangeable.
