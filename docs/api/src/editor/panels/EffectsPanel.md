# src/editor/panels/EffectsPanel.tsx

The Effects rail panel: quick-add pills for timeline elements (zoom/spotlight/layout/camera-move,
via `EffectPills`), click-ripple controls, always-on spotlight controls, and (as of Task 26) the
video-effect mode picker - the full set of `doc.settings.clickfx` (`ClickFxSettings`) controls,
matching the HUD's own `SettingsClickFx` one-for-one.

## EffectsPanel

```tsx
export function EffectsPanel({ settings, onChange, onClose, onAddZoom, onAddSpotlight, onAddLayout, onAddCameraMove }: {
  settings: ClickFxSettings; onChange: (v: ClickFxSettings) => void; onClose: () => void;
  onAddZoom: () => void; onAddSpotlight: () => void; onAddLayout: () => void; onAddCameraMove: () => void;
}): JSX.Element
```

### Props

- `settings: ClickFxSettings` - the current per-project click-fx/spotlight/video-fx settings (`doc.settings.clickfx`).
- `onChange: (v: ClickFxSettings) => void` - called with the full next `ClickFxSettings` on every control change, via the panel's own `set(k, v)` helper (`{ ...settings, [k]: v }`) - the standard per-panel idiom.
- `onClose: () => void` - closes the panel (back to the AI tab).
- `onAddZoom` / `onAddSpotlight` / `onAddLayout` / `onAddCameraMove: () => void` - add-at-playhead callbacks passed straight through to `EffectPills`; this panel doesn't know *how* each element gets added, only that the pill was clicked.

### Behavior

**Click ripples.** Gated behind the `enabled` `.e-switchrow`. When on: `Ripple Style` (a 7-way `Picker` - `none`/`ripple`/`pulse`/`glow`/`shockwave`/`particles`/`neon`), `Ripple Color` (a 5-swatch `Swatches` row, `variant="small"`, writing `settings.color`), and `Intensity` (a `0.2`-`1.0` slider writing `settings.intensity`). **Style-dependent disabling (Task 26):** when `settings.style === "none"`, Ripple Color and Intensity are `disabled` (not hidden) - there's nothing for a color or intensity to apply to with no ripple style, but hiding them the instant "None" is picked would also hide the very picker that got the user there, so they stay visible-but-inert instead.

**Spotlight.** Gated behind the `spotlight` `.e-switchrow` ("Spotlight always on"). When on: `Spotlight Mode` (6-way `Picker`, mirrors `SettingsClickFx`'s `MODES`), `Tint` (Task 26 - a 5-swatch `Swatches` row writing `settings.spotlight_tint`, the exact same palette as the HUD's `TINTS`), `Dim Override` / `Radius Override` (existing sliders), `Feather` (Task 26 - `0.02`-`0.25` slider writing `settings.spotlight_feather`, same range as the HUD), and a `Dim webcam` `.e-switchrow` writing `settings.spotlight_dim_camera`.

**Video effect (Task 26).** A separate, always-visible section - NOT gated behind `spotlight` or `enabled`, since it's an independent hotkey-activated overlay in the backend (`settings.clickfx.video_fx_mode`, `VideoFxMode`). One `Video FX Mode` picker (`nebulawash`/`cinematicdim`/`screenfocus`/`colorpop`), mirroring the HUD's `SettingsClickFx` `VMODES` exactly (same 4 values, same labels, same order) so the mode reads the same whichever panel set it.

### Notes

- `STYLES`, `MODES`, `VMODES` are module-level constants kept in lockstep with `src/hud/settings/SettingsClickFx.tsx`'s equivalents by convention (not by shared import) - if the HUD's palette/mode list changes, this file's copies need updating too.
- `SWATCHES`/`TINTS` are `[color, human name][]` tuple arrays (Task 26) - the name becomes each swatch's `aria-label`. `TINTS`' color VALUES still mirror the HUD's `TINTS` exactly; the names themselves are local to this file (the HUD's own swatches use the bare `rgb(...)` string as their label, not a name).
- `swatchItems(colors)` is a small local adapter from `[[number,number,number], string][]` to `SwatchItem<[number,number,number]>[]` (`{key, css, value, ariaLabel}`, see `Swatches.md`) - both `Ripple Color` and `Tint` call it with their own array.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "effects"` panel, wired to `doc.settings.clickfx` / `saveDocSettings`, with the four add-callbacks threaded from `Editor`.
