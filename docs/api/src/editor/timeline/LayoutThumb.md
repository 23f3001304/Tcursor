# src/editor/timeline/LayoutThumb.tsx

The T34 L4 arrangement schematic, drawn as a tiny inline SVG - two tokens-only rects (an `--e-layout`-outlined screen frame, a filled `--e-cam` webcam dot) built from `arrThumb.ts`'s geometry. Shared verbatim by the Layout lane's pill (`layoutLane.tsx`, 24x14 - its own icon slot) and `LayoutInspector`'s header (48x28) - same `viewBox`, just a bigger rendered `width`/`height`, so the two can never draw a different schematic for the same segment.

## LayoutThumb

```tsx
export function LayoutThumb({ panels, w = THUMB_W, h = THUMB_H, className = "e-laythumb" }: {
  panels: ResolvedPanels | null; w?: number; h?: number; className?: string;
}): JSX.Element
```

### Props

- `panels: ResolvedPanels | null` - a segment's resolved screen/cam panels (`resolvedPanelsFor`, T34 L2), or `null` while presets haven't loaded / there's no segment.
- `w`/`h` - the rendered SVG size in CSS px; the underlying geometry is always computed in `arrThumb.ts`'s fixed `THUMB_W`x`THUMB_H` (24x14) coordinate space and scaled up by the `<svg>`'s own `width`/`height` against a fixed `viewBox` - never recomputed per size.
- `className` - defaults to `e-laythumb`, the class the Layout lane's own narrow-pill `@container` rule targets (`.e-layblk .e-laythumb`, `editor.css`) to hide the thumbnail before the pill's text. `LayoutInspector`'s header instance keeps the same default - harmless there, since it isn't inside a `.e-layblk`.

### Behavior

Calls `arrThumb(panels)`. If BOTH boxes come back `null` (no resolved panels at all - the `panels === null` case, since a real `ResolvedPanels` always has at least one visible panel per the backend's own clamp), renders the plain `IconAspectRatio` (12px) the layout pill showed before this task, rather than an empty gap in the label. Otherwise renders an `<svg viewBox="0 0 24 14">` with one `<rect className="e-laythumb-screen">` for `t.screen` and one `<rect className="e-laythumb-cam">` for `t.cam`, each only when its box is non-`null` (a hidden panel draws nothing, not a zero-size rect). `aria-hidden` - the pill/inspector's own text label already carries the meaning ("Custom" / the preset name); the thumbnail is decorative reinforcement, not additional information a screen reader needs to announce separately.

### Used by

- `src/editor/timeline/layoutLane.tsx` - `layoutLabel`, the Layout lane pill's `RegionRows.renderLabel`, at the default 24x14 size.
- `src/editor/inspectors/LayoutInspector.tsx` - `PanelHeader`'s new `thumb` slot, at 48x28, fed the SAME `panels` value the inspector already computes for its visibility switches (`resolvedPanelsFor(seg, presets)`), so the header schematic and the switches below can never disagree.
