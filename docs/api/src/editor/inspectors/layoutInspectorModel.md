# src/editor/inspectors/layoutInspectorModel.ts

The Layout inspector's pure model - the `GraphInput` its Motion section draws (`layoutGraphInput`), the four presets its "Start from" row offers (`LAYOUT_PRESETS`), and the preset-name prettifier (`prettyLayout`). Split out of `LayoutInspector.tsx` so that file is only the panel; it mirrors `zoomInspectorModel.ts`, which does the same job for the Zoom inspector.

## LAYOUT_PRESETS

```ts
export const LAYOUT_PRESETS: { value: LayoutPresetName; label: string }[]
```

The four presets the Composition section's "Start from" row offers, in row order: Camera, Presenter, Screen only, Camera only. `"screen"` is deliberately absent - it is the empty default a segment falls back to, not something you start from.

## prettyLayout

```ts
export const prettyLayout = (v: string) => string
```

A preset id as a label: underscores to spaces, first letter capitalised (`screen_only` -> `Screen only`). Used for the "Custom, based on X" readout on the Composition section's heading.

## layoutGraphInput

```ts
export function layoutGraphInput(seg: LayoutSeg, segs: LayoutSeg[]): GraphInput
```

The segment as the graph draws it: lane `"layout"`, progress 0 to 1 over `transition_ms` on `easing`, the hold, 1 to 0 over `transition_out_ms` on `easing_out`, and the nearest segments before and after (`segs`, the whole lane from `PropertiesSlot`) as the ghosts.
