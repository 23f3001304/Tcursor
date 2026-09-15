# src/editor/panels/layout/LayoutMiniPreview.tsx

The picked layout's schematic under the Layouts panel's picker: the screen panel and the webcam panel, drawn from that layout's `ModeAppearance` at the panel's own width.

## LayoutMiniPreview

```tsx
export function LayoutMiniPreview({ mode, ma }: { mode: ModeKey; ma: ModeAppearance }): JSX.Element
```

A wrapper, deliberately - about five lines of it. The boxes are drawn by `LayoutPreview` (`src/hud/components/LayoutPreview.tsx`), the same pure component the HUD's Appearance settings use, rather than by a second implementation of the same arithmetic. That arithmetic (padding, screen scale, bubble corner and margins, the 16:9 stage, `cam_size` as a height fraction converted to both axes) is the one thing that must not drift between the two editors of the same five layouts, so there is one copy of it and both call it.

### What the wrapper is actually for

The CSS. `LayoutPreview` emits `.lp-stage` / `.lp-screen` / `.lp-cam`, whose rules live in `src/hud/settings/settings.css` - a stylesheet the editor does not load, and which is written in HUD tokens (`--surface`, `--muted`, `--line`) that do not exist there either. `.e-laypv` in `panels.css` restates them in editor tokens at a two-class specificity, so the schematic is styled in the editor and still wins wherever both stylesheets end up in one bundle:

- the stage is `--e-bg` with the panel's inset highlight, at a fixed 16:9 (`padding-top: 56.25%`);
- the screen panel is `--e-plate`, the neutral plate the panels already use behind glyph artwork;
- the webcam panel is `--e-cam`, the timeline's own camera-lane accent - the same identity colour the camera pill carries, so the two boxes are told apart by meaning rather than by a stroke.

`aria-hidden` - it is a picture of the controls right below it, and every value it draws is already announced by the sliders and segments themselves.

### Used by

- `src/editor/panels/layout/LayoutsPanel.tsx`.
