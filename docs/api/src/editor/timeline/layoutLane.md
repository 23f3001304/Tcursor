# src/editor/timeline/layoutLane.tsx

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

`doc.layout` minus the empty `"screen"` default (the one layout hidden as a pill - deleting a pill, or leaving a gap, is how you get back to it), laid into non-overlapping rows (`layoutRegions`, `./layers.ts`), each entry then getting a `panels: resolvedPanelsFor(l, presets) ?? null` - `null` only while `presets` itself is `null` (the same gate `LayoutInspector` uses), never per-segment, since `resolvedPanelsFor` always resolves something once presets have loaded.

Two nested `useMemo`s: the `"screen"` filter keyed on `layout` alone, the panel-attaching map keyed on `[nonScreen, presets]` - so a `layoutPresets` refetch that lands with the SAME resolved rects (the common case: most edits don't touch layout) doesn't force a new `layouts` array reference either, preserving `RegionRows`' memo the same way the pre-T34-L4 bare `useMemo(() => layoutRegions(nonScreenLayout), [nonScreenLayout])` did.

### Used by

`Timeline.tsx` - `const layouts = useLayoutLaneRegions(doc.layout, layoutPresets);`, replacing that file's former inline `nonScreenLayout`/`layouts` memo pair.

## layoutLabel

```tsx
export const layoutLabel = (l: LayoutRegion) => JSX.Element
```

`RegionRows.renderLabel` for the Layout lane (T34 L4): a `LayoutThumb` (`./LayoutThumb.tsx`) built from `l.panels`, followed by "Custom" for a segment carrying its own `arrangement`, else `prettyLayout(l.layout)` - replacing the plain `IconAspectRatio` + preset-name label this lane showed before this task.

Stays a stable MODULE-scope function, not a per-render closure: `panels` rides on the region object itself (`useLayoutLaneRegions`, above) rather than being captured from a `layoutPresets` prop, which is exactly why that hook exists - a closure over the live `layoutPresets` would have needed a fresh `renderLabel` identity every time presets changed, defeating `RegionRows`' `React.memo` the same way a fresh inline arrow would (see `Timeline.md`'s "Render hygiene").

## layoutExtraStyle

```ts
export const layoutExtraStyle = (l: { transition_ms: number; transition_out_ms: number }, s: number, e: number) => React.CSSProperties
```

Unchanged from its pre-T34-L4 home in `Timeline.tsx`: the `--fin`/`--fout` fade-ramp CSS vars (`transitionRampPct`, `./layers.ts`) `RegionRows` merges onto the layout pill's inline style, driving `.e-layblk::before`/`::after`'s gradient widths (`editor.css`).
