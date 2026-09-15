# src/editor/motion/graphCoords.ts

The motion graph's plot coordinates: sampling a ramp's curve into them (`sampleRamp`, `graphKeys`), and reading a point back out of a built model (`msToX`, `xToMs`, `progressToY`, `yToProgress`, `clientToPlot`). Split out of `graphModel.ts`, which keeps the model types and `buildGraph` itself - `buildGraph` calls into here on the way in, and `useGraphDrag` calls into here on the way back out, so a drag and the curve it is dragging share one set of mappings. Nothing here touches React, and only types flow back from `graphModel.ts`, so there is no runtime cycle between the two.

## r2

```ts
export const r2 = (v: number) => number
```

Rounds a plot coordinate to two decimals. Every number that reaches an SVG path string goes through it, so a rebuild of the same curve produces a byte-identical `d` and React skips the attribute write.

## SAMPLES_PER_RAMP

```ts
export const SAMPLES_PER_RAMP = 64;
```

Samples per ramp, so a polyline has 65 points. At 256 px of plot that is a point every four pixels: past the eye's ability to see a corner, and cheap enough to rebuild the whole model on every pointer move during a drag.

## sampleRamp

```ts
function sampleRamp(spec, k, base, peak, which): number[]
```

One ramp's motion values. A curve already in keys form is parsed ONCE by the caller and evaluated here through `evalKeys`; everything else goes through `ease`, which parses its string on every call - at 65 samples a ramp, on every frame of a drag, that difference is the whole reason the two paths exist.

The out ramp runs the curve backwards (`1 - e`): its progress 1 is the REST value, not the peak. Every other mapping in this file repeats that same inversion, and `progressToY` is where a caller gets it for free.

## graphKeys

```ts
function graphKeys(k, x0, x1, which, base, peak, yPx): GraphKey[]
```

The dots and the ends of their tangent arms, in pixels. A handle is an offset from its key in curve units, so it maps through exactly the same transform the key does - which is what makes a handle drag land where the pointer is even on the mirrored out ramp.

## msToX

```ts
export const msToX = (m: GraphModel, input: GraphInput, ms: number) => number
```

A timeline instant as a plot x. Exact at both ends: `startMs` is `plot.x` and `endMs` is `plot.x + plot.w`.

## xToMs

```ts
export const xToMs = (m: GraphModel, input: GraphInput, x: number) => number
```

The inverse, used by a retime drag to turn a pointer into a duration. It round-trips with `msToX` to floating-point exactness, which `graphModel.test.ts` pins - a retime that landed a millisecond off the key the user grabbed would creep by one each drag.

## progressToY

```ts
export const progressToY = (m: GraphModel, which: "in" | "out", v: number) => number
```

A ramp progress (a key's `v`) as a y pixel, via the two `yTicks` rails. The out ramp's mirror is applied here, once.

## yToProgress

```ts
export const yToProgress = (m: GraphModel, which: "in" | "out", y: number) => number
```

The inverse: what a pointer's y means as ramp progress. Unclamped on purpose - dragging a key above the peak rail is how you ask for an overshoot.

## clientToPlot

```ts
export function clientToPlot(m, rect, clientX, clientY): [number, number]
```

A pointer's client point in the model's own coordinates. The svg is drawn through a viewBox so it can be laid out at any CSS width, and this is the single place that scale is undone; every drag and the double-click add go through it.
