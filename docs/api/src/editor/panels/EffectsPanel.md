# src/editor/panels/EffectsPanel.tsx

The Effects rail panel: quick-add pills for timeline elements (zoom/spotlight/the three masks/layout/camera-move,
via `EffectPills`), click-ripple controls, and - under its one `Disclosure` - the always-on spotlight
and video-effect controls that now live in `SpotlightSection.tsx`. Together they are the full set of
`doc.settings.clickfx` (`ClickFxSettings`) controls, matching the HUD's own `SettingsClickFx`
one-for-one.

## EffectsPanel

```tsx
export function EffectsPanel({ settings, onChange, onClose, onAddZoom, onAddSpotlight, onAddMask, onAddLayout, onAddCameraMove, onAddText }: {
  settings: ClickFxSettings; onChange: (v: ClickFxSettings) => void; onClose: () => void;
  onAddZoom: () => void; onAddSpotlight: () => void; onAddMask: (kind: MaskKind) => void;
  onAddLayout: () => void; onAddCameraMove: () => void; onAddText: (kind: TextKind) => void;
}): JSX.Element
```

### Props

- `settings: ClickFxSettings` - the current per-project click-fx/spotlight/video-fx settings (`doc.settings.clickfx`).
- `onChange: (v: ClickFxSettings) => void` - called with the full next `ClickFxSettings` on every control change, via the panel's own `set(k, v)` helper (`{ ...settings, [k]: v }`) - the standard per-panel idiom. `set` is also handed down to `SpotlightSection`, so this file is the only place that knows how one key becomes a whole next object.
- `onClose: () => void` - closes the panel (back to the AI tab).
- `onAddZoom` / `onAddSpotlight` / `onAddLayout` / `onAddCameraMove: () => void` - add-at-playhead callbacks passed straight through to `EffectPills`; this panel doesn't know *how* each element gets added, only that the pill was clicked.
- `onAddText: (kind: TextKind) => void` - the same, for the four text pills. It is the one that takes an argument, because a text item is seeded from its kind.

### Behavior

**Flow (panel pass, 2026-09-13).** Four groups: the **Insert timeline elements** pills first (the panel's primary act), then **Clicks** (the Click animations switch with Ripple Style / Color / Intensity under it), then **Spotlight** (the Spotlight switch with its six controls under it), then **Video effect** last. Same controls, same names, same order within each group; what changed is that each switch and the controls it enables are now one group rather than a `.e-sec` divider with the dependents floating after it, and each group has a heading.

**Height (usability pass, 2026-09-13).** With both switches on, the four groups came to about 1064px against a 620px slot. Two changes:

1. **Two-up rows** (`.e-two`) inside the disclosure - see `SpotlightSection.md`. Ripple Style | Ripple Color was a two-up too until the panel pass of 2026-09-15: at the narrow density the five swatches wrapped into a 4+1 row and "Color pop" ellipsised in the dropdown, so the two are full-width fields again, 50px dearer and no longer orphaned.
2. **Spotlight and Video effect went under the panel's one `Disclosure`.** They are the two least-used groups: a spotlight you want for one moment is the Spotlight Highlight pill at the top of this panel, so that group is only the always-on version, and the video effect is hotkey-driven. Both are set once per project at most, and together they are about 450px, which is more than the panel's whole budget.

Measured by rows at 320px with Click animations on: padding 36, header 53, the four pills 229, gap 16, Clicks 180, gap 16, the More row 28 - **558**. With the switch off, 431.

**Click ripples.** Gated behind the `enabled` `.e-switchrow`. When on: `Ripple Style` (a 7-way `Picker` - `none`/`ripple`/`pulse`/`glow`/`shockwave`/`particles`/`neon`), `Ripple Color` (a 5-swatch `Swatches` row over `RIPPLE_COLORS`, writing `settings.color`), and `Intensity` (a `0.2`-`1.0` slider writing `settings.intensity`). **Style-dependent disabling (Task 26):** when `settings.style === "none"`, Ripple Color and Intensity are `disabled` (not hidden) - there's nothing for a color or intensity to apply to with no ripple style, but hiding them the instant "None" is picked would also hide the very picker that got the user there, so they stay visible-but-inert instead.

### Notes

- `STYLES` is a module-level constant kept in lockstep with `src/hud/settings/SettingsClickFx.tsx`'s equivalent by convention (not by shared import) - if the HUD's list changes, this file's copy needs updating too. The spotlight and video-fx lists moved with their section; see `SpotlightSection.md`.
- The palettes and the `Swatches` adapter are in `effectSwatches.md`.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "effects"` panel, wired to `doc.settings.clickfx` / `saveDocSettings`, with the four add-callbacks threaded from `Editor`.
