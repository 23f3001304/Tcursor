# src/editor/panels/SpotlightSection.tsx

The two groups the Effects panel keeps under its `Disclosure`: the always-on **Spotlight** and the **Video effect** mode. Split out of `EffectsPanel.tsx` so that file is the panel's own flow (header, quick-add pills, clicks) and this one is what folds away under **More**.

## SpotlightSection

```tsx
export function SpotlightSection({ settings, set }: {
  settings: ClickFxSettings;
  set: <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => void;
}): JSX.Element
```

### Props

- `settings: ClickFxSettings` - the same per-project slice the whole Effects panel edits (`doc.settings.clickfx`).
- `set` - the panel's own one-key patch helper, passed down rather than re-derived. This section never calls `onChange` itself, so there is exactly one place (`EffectsPanel`) that knows how a change becomes a whole next `ClickFxSettings`.

### Behavior

**Spotlight.** Gated behind the `spotlight` `.e-switchrow` ("Spotlight always on"). When on: `Spotlight Mode` (6-way `Picker` over `MODES`, mirroring `SettingsClickFx`'s), `Tint` (a 5-swatch row over `SPOTLIGHT_TINTS`, writing `settings.spotlight_tint`), `Dim Override` / `Radius Override` sliders, a `Feather` slider (`0.02`-`0.25`, writing `settings.spotlight_feather`, the HUD's range), and a `Dim webcam` `.e-switchrow` writing `settings.spotlight_dim_camera`.

This is only the ALWAYS-ON spotlight. A spotlight you want for one moment is the Spotlight Highlight pill at the top of the panel, which adds a timeline element instead.

**Video effect.** NOT gated behind `spotlight` or `enabled` - it is an independent hotkey-activated overlay in the backend (`settings.clickfx.video_fx_mode`, `VideoFxMode`). One `Video FX Mode` picker over `VMODES` (`nebulawash`/`cinematicdim`/`screenfocus`/`colorpop`), mirroring the HUD's list exactly, same four values, same labels, same order, so the mode reads the same whichever panel set it.

*Why this one stayed a dropdown.* The panel pass converts 2-to-4 exclusive states to `Segmented`, and this is four - but "Nebula wash" and "Screen focus" are two words each, and four-up at 320px gives a segment about 66px. Spotlight Mode (six) is over the limit outright. Both stay dropdowns.

**Two-up rows** (`.e-two`) pair Spotlight Mode | Tint and Dim Override | Radius Override. Each pair saves roughly 50px against a 620px slot, and none of those four labels is long enough to ellipsise.

### Notes

- `MODES` and `VMODES` are module-level constants kept in lockstep with `src/hud/settings/SettingsClickFx.tsx`'s equivalents by convention (not by shared import) - if the HUD's mode list changes, this file's copies need updating too.
- The palettes and the `Swatches` adapter live in `effectSwatches.ts`, shared with the clicks half.

### Used by

- `src/editor/panels/EffectsPanel.tsx` - the whole body of its one `Disclosure`.
