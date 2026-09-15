# src/editor/timeline/lanes/LayoutLane.tsx

The Layout lane's own presentation logic for `Timeline.tsx`: which segments show as pills, laid into rows and carrying their resolved panels, plus the `renderLabel`/`extraStyle` `RegionRows` render-props those pills use. Split out of `Timeline.tsx` (T34 L4) so that file - already at its 200-line cap - didn't have to grow to add the thumbnail; the zoom/FX lanes' own (simpler, closure-free) `zoomLabel`/`fxLabel` stayed in `Timeline.tsx` since they needed no such split.

## LayoutRegion

```ts
export type LayoutRegion = LayoutSeg & { layer: number; panels: ResolvedPanels | null };
```

A layout segment after `layoutRegions` row-assignment, carrying its own resolved `panels` - the thumbnail source `layoutLabel` reads, attached once here rather than re-derived inside the render-prop itself.

## useLayoutLaneRegions

```ts
export function useLayoutLaneRegions(layout: LayoutSeg[], presets: LayoutPresets | null): LayoutRegion[]
```

`doc.layout` minus the empty `"screen"` default (the one layout hidden as a pill - deleting a pill, or leaving a gap, is how you get back to it), laid into non-overlapping rows (`layoutRegions`, `../model/layers.ts`), each entry then getting a `panels: resolvedPanelsFor(l, presets) ?? null` - `null` only while `presets` itself is `null` (the same gate `LayoutInspector` uses), never per-segment, since `resolvedPanelsFor` always resolves something once presets have loaded.

Two nested `useMemo`s: the `"screen"` filter keyed on `layout` alone, the panel-attaching map keyed on `[nonScreen, presets]` - so a `layoutPresets` refetch that lands with the SAME resolved rects (the common case: most edits don't touch layout) doesn't force a new `layouts` array reference either, preserving `RegionRows`' memo the same way the pre-T34-L4 bare `useMemo(() => layoutRegions(nonScreenLayout), [nonScreenLayout])` did.

### Used by

`Timeline.tsx` - `const layouts = useLayoutLaneRegions(doc.layout, layoutPresets);`, replacing that file's former inline `nonScreenLayout`/`layouts` memo pair.

## layoutLabel

```tsx
export const layoutLabel = (l: LayoutRegion) => JSX.Element
```

`RegionRows.renderLabel` for the Layout lane (T34 L4): a `LayoutThumb` (`./LayoutLane.tsx`) built from `l.panels`, followed by "Custom" for a segment carrying its own `arrangement`, else `prettyLayout(l.layout)` - replacing the plain `IconAspectRatio` + preset-name label this lane showed before this task.

Stays a stable MODULE-scope function, not a per-render closure: `panels` rides on the region object itself (`useLayoutLaneRegions`, above) rather than being captured from a `layoutPresets` prop, which is exactly why that hook exists - a closure over the live `layoutPresets` would have needed a fresh `renderLabel` identity every time presets changed, defeating `RegionRows`' `React.memo` the same way a fresh inline arrow would (see `Timeline.md`'s "Render hygiene").

## layoutExtraStyle

```ts
export const layoutExtraStyle = (l: { transition_ms: number; transition_out_ms: number }, s: number, e: number) => React.CSSProperties
```

Unchanged from its pre-T34-L4 home in `Timeline.tsx`: the `--fin`/`--fout` fade-ramp CSS vars (`transitionRampPct`, `../model/layers.ts`) `RegionRows` merges onto the layout pill's inline style, driving `.e-layblk::before`/`::after`'s gradient widths (`timeline.css`).

## LayoutThumb

```tsx
export function LayoutThumb({ panels, w = THUMB_W, h = THUMB_H, className = "e-laythumb" }: {
  panels: ResolvedPanels | null; w?: number; h?: number; className?: string;
}): JSX.Element
```

### Props

- `panels: ResolvedPanels | null` - a segment's resolved screen/cam panels (`resolvedPanelsFor`, T34 L2), or `null` while presets haven't loaded / there's no segment.
- `w`/`h` - the rendered SVG size in CSS px; the underlying geometry is always computed in `arrThumb.ts`'s fixed `THUMB_W`x`THUMB_H` (24x14) coordinate space and scaled up by the `<svg>`'s own `width`/`height` against a fixed `viewBox` - never recomputed per size.
- `className` - defaults to `e-laythumb`, the class the Layout lane's own narrow-pill `@container` rule targets (`.e-layblk .e-laythumb`, `timeline.css`) to hide the thumbnail before the pill's text. `LayoutInspector`'s header instance keeps the same default - harmless there, since it isn't inside a `.e-layblk`.

### Behavior

Calls `arrThumb(panels)`. If BOTH boxes come back `null` (no resolved panels at all - the `panels === null` case, since a real `ResolvedPanels` always has at least one visible panel per the backend's own clamp), renders the plain `IconAspectRatio` (12px) the layout pill showed before this task, rather than an empty gap in the label. Otherwise renders an `<svg viewBox="0 0 24 14">` with one `<rect className="e-laythumb-screen">` for `t.screen` and one `<rect className="e-laythumb-cam">` for `t.cam`, each only when its box is non-`null` (a hidden panel draws nothing, not a zero-size rect). `aria-hidden` - the pill/inspector's own text label already carries the meaning ("Custom" / the preset name); the thumbnail is decorative reinforcement, not additional information a screen reader needs to announce separately.

### Used by

- `src/editor/timeline/lanes/LayoutLane.tsx` - `layoutLabel`, the Layout lane pill's `RegionRows.renderLabel`, at the default 24x14 size.
- `src/editor/inspectors/LayoutInspector.tsx` - `PanelHeader`'s new `thumb` slot, at 48x28, fed the SAME `panels` value the inspector already computes for its visibility switches (`resolvedPanelsFor(seg, presets)`), so the header schematic and the switches below can never disagree.
