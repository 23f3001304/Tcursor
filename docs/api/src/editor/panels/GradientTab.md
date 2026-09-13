# src/editor/panels/GradientTab.tsx

`BackgroundPanel`'s Gradient tab: the twelve curated presets on top, the user's own gradient below. Split out of `BackgroundPanel.tsx` for the 200-line budget, and because the custom controls are a self-contained concern the other two tabs share nothing with.

Both halves write the SAME four settings fields (`gradient_from` / `gradient_mid` / `gradient_to` / `gradient_angle_deg`), so picking a preset and then nudging one stop is a single continuous edit rather than two modes. *Why no separate "custom" flag:* the preset grid simply stops showing a selection once the colours no longer match any preset, which is both the honest state and one less thing to persist.

## isPreset

```ts
export function isPreset(t: BackgroundThumb, bg: BackgroundSettings): boolean
```

Is `bg` exactly this preset? Every stop AND the angle must agree, and the middle stop's ABSENCE counts as a value: a preset the user has since edited (or whose middle stop they removed) correctly reads as "no preset selected".

Treats `gradient_mid: null` and a missing `gradient_mid` as the same thing, because Rust omits the key entirely when the stop is unset - so a doc that round-tripped through the backend and one built in TypeScript compare equal.

A wallpaper tile (no `gradient` field at all) never matches.

### Behaviors

- `matches only when every stop AND the angle agree`.
- `treats a removed middle stop as a difference, whether it is null or absent`.
- `never matches a wallpaper tile, which carries no stops at all`.

## GradientTab

```tsx
export function GradientTab({ bg, thumbs, setBg }: {
  bg: BackgroundSettings;
  thumbs: BackgroundThumb[];
  setBg: (patch: Partial<BackgroundSettings>) => void;
}): JSX.Element
```

### Behavior

**Presets** - the twelve gradient tiles from `background_thumbs`, through the shared `WallpaperRow` (`WallpaperGrid.md`): one strip labelled Presets, in the same `.e-rowstack` idiom the Wallpapers tab uses, which is 55px where the wrapping grid was 287. The row's label IS the heading, so there is no `.e-sechead` above it saying the same word. Selecting a tile applies its stops AND its angle in a single patch, including `gradient_mid: null` for a two-stop preset (so switching from a three-stop preset actually drops the middle stop instead of leaving it behind). The selection ring is computed with `isPreset`, and is `null` whenever `bg.kind` is not `"gradient"`.

**Custom** - a `.e-colorrow` of `ColorInput` swatches: From, Middle (only when a middle stop exists), To. Under them, one `.e-ghostbtn` toggles the middle stop. Adding one starts it at the AVERAGE of the two ends, so switching it on changes nothing visible until the user moves it - the control announces itself without silently restyling the background.

Every edit also asserts `kind: "gradient"`, so tweaking a colour while another kind is active switches to the gradient instead of quietly changing a background nobody can see.

**Gradient Angle** - a `Slider` over `0..360` in 5-degree steps, formatted as `"<n> deg"`. It inherits `Slider`'s own commit debounce, which matters here because each commit is an `apply_edit_op` round trip that rebuilds the preview background. Since the panel pass it also inherits the click-to-type readout, which matters even more here: 360 degrees across a 290px track is under one degree per pixel, so an exact angle was previously only reachable by counting arrow-key presses. It sits in the Custom group, with the stops it applies to.

### Used by

- `src/editor/panels/BackgroundPanel.tsx` - rendered for `tab === "gradient"`.
