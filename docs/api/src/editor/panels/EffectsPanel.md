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

**Flow (panel pass, 2026-09-13).** Four groups: the **Insert timeline elements** pills first (the panel's primary act), then **Clicks** (the Click animations switch with Ripple Style / Color / Intensity under it), then **Spotlight** (the Spotlight switch with its six controls under it), then **Video effect** last. Same controls, same names, same order within each group; what changed is that each switch and the controls it enables are now one group rather than a `.e-sec` divider with the dependents floating after it, and each group has a heading.

**Height (usability pass, 2026-09-13).** With both switches on, the four groups came to about 1064px against a 620px slot. Two changes:

1. **Two-up rows** (`.e-two`) for Ripple Style | Ripple Color and, inside the disclosure, Spotlight Mode | Tint and Dim Override | Radius Override. Each pair saves roughly 50px, and none of the four labels is long enough to ellipsise.
2. **Spotlight and Video effect went under the panel's one `Disclosure`.** They are the two least-used groups: a spotlight you want for one moment is the Spotlight Highlight pill at the top of this panel, so this group is only the always-on version, and the video effect is hotkey-driven. Both are set once per project at most, and together they are about 450px, which is more than the panel's whole budget.

Measured by rows at 320px with Click animations on: padding 36, header 53, the four pills 229, gap 16, Clicks 180, gap 16, the More row 28 - **558**. With the switch off, 431.

**Click ripples.** Gated behind the `enabled` `.e-switchrow`. When on: `Ripple Style` (a 7-way `Picker` - `none`/`ripple`/`pulse`/`glow`/`shockwave`/`particles`/`neon`), `Ripple Color` (a 5-swatch `Swatches` row, `variant="small"`, writing `settings.color`), and `Intensity` (a `0.2`-`1.0` slider writing `settings.intensity`). **Style-dependent disabling (Task 26):** when `settings.style === "none"`, Ripple Color and Intensity are `disabled` (not hidden) - there's nothing for a color or intensity to apply to with no ripple style, but hiding them the instant "None" is picked would also hide the very picker that got the user there, so they stay visible-but-inert instead.

**Spotlight** (under **More**). Gated behind the `spotlight` `.e-switchrow` ("Spotlight always on"). When on: `Spotlight Mode` (6-way `Picker`, mirrors `SettingsClickFx`'s `MODES`), `Tint` (Task 26 - a 5-swatch `Swatches` row writing `settings.spotlight_tint`, the exact same palette as the HUD's `TINTS`), `Dim Override` / `Radius Override` (existing sliders), `Feather` (Task 26 - `0.02`-`0.25` slider writing `settings.spotlight_feather`, same range as the HUD), and a `Dim webcam` `.e-switchrow` writing `settings.spotlight_dim_camera`.

**Video effect (Task 26).** A separate section (under **More** since the usability pass) - NOT gated behind `spotlight` or `enabled`, since it's an independent hotkey-activated overlay in the backend (`settings.clickfx.video_fx_mode`, `VideoFxMode`). One `Video FX Mode` picker (`nebulawash`/`cinematicdim`/`screenfocus`/`colorpop`), mirroring the HUD's `SettingsClickFx` `VMODES` exactly (same 4 values, same labels, same order) so the mode reads the same whichever panel set it.

*Why this one stayed a dropdown.* The panel pass converts 2-to-4 exclusive states to `Segmented`, and this is four - but "Nebula wash" and "Screen focus" are two words each, and four-up at 320px gives a segment about 66px. Ripple Style (seven) and Spotlight Mode (six) are over the limit outright. All three stay dropdowns.

### Notes

- `STYLES`, `MODES`, `VMODES` are module-level constants kept in lockstep with `src/hud/settings/SettingsClickFx.tsx`'s equivalents by convention (not by shared import) - if the HUD's palette/mode list changes, this file's copies need updating too.
- `SWATCHES`/`TINTS` are `[color, human name][]` tuple arrays (Task 26) - the name becomes each swatch's `aria-label`. `TINTS`' color VALUES still mirror the HUD's `TINTS` exactly; the names themselves are local to this file (the HUD's own swatches use the bare `rgb(...)` string as their label, not a name).
- `swatchItems(colors)` is a small local adapter from `[[number,number,number], string][]` to `SwatchItem<[number,number,number]>[]` (`{key, css, value, ariaLabel}`, see `Swatches.md`) - both `Ripple Color` and `Tint` call it with their own array.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "effects"` panel, wired to `doc.settings.clickfx` / `saveDocSettings`, with the four add-callbacks threaded from `Editor`.
