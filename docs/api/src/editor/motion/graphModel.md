# src/editor/motion/graphModel.ts

The picture behind `MotionGraph`: a region's whole motion - the in ramp, the hold, the out ramp, the neighbours' handoffs and the two axes - reduced to pixels. Pure, so `graphModel.test.ts` pins it with no DOM at all.

**The graph IS the motion.** Every sample runs through the one evaluator the preview and the export share (`ease` in `timeline/model/layoutTrack.ts`, or `evalKeys` on an already-parsed curve, which is the same arm of the same function). A spring, a named curve, a cubic and a hand-drawn `keys(...)` therefore all draw through one code path, and the drawing cannot show a motion the render would not produce (`docs/superpowers/specs/2026-09-15-motion-editor-design.md` 4).

**The coordinate system.** `plot` is the SPAN's own rectangle: `msToX(startMs)` is `plot.x` and `msToX(endMs)` is `plot.x + plot.w`, exactly. A `GHOST_PX` gutter sits outside it on each side, which is where the neighbours' handoffs are drawn - "outside the span" is a real position here, not a metaphor - and the y label column sits outside that again. The y axis is the motion's VALUE (a zoom's scale, everything else's progress), and its range is computed from the samples, so an overshoot or an anticipation gets room instead of being clipped.

## Lane

```ts
export type Lane = "zoom" | "layout" | "cam" | "text"
```

Which of the four regions the graph is drawing. It picks the accent (`--e-zoom` / `--e-layout` / `--e-cam` / `--e-text`, the timeline lanes' own tokens), the axis labels (`x` for a scale, `%` for a progress) and the rest value.

`"text"` (Batch 2c) rests at 0 and peaks at 1 like `layout` and `cam`, because a text item's graph IS its alpha: it comes up from nothing, holds, and goes back to nothing. Only `zoom` rests at 1 (`baseOf`), because a zoom that is not zooming is still showing the picture at 1x.

## RampSpec

```ts
export interface RampSpec { easing: string; durMs: number }
```

One ramp: the curve it runs on and how long it lasts. The two fields come from the doc side by side (`easing` + `zoom_in_ms`, `easing_out` + `transition_out_ms`), which is why the graph can retime a ramp and reshape it in the same gesture.

## GraphInput

```ts
export interface GraphInput { lane; startMs; endMs; rampIn; rampOut; peak; followHint?; prev?; next? }
```

Everything the drawing needs, in terms an inspector already has:

- `startMs` / `endMs` - the region's span. A camera move has no span of its own, so the caller passes the previous keyframe's `t_ms` (or `endMs - KF_BLEND_MS` for the first one, the window `camMoveAt` actually blends over).
- `rampOut` is `null` for a camera move, which is one ramp into a pose and has no return.
- `peak` is `target_scale` for a zoom (the ramps run `1.0 -> peak -> 1.0`) and `1` for the other two (`0 -> 1 -> 0`).
- `followHint` marks a zoom whose target is the cursor: the hold is not a value at all there, it is the follow filter, and the graph says so under the plateau instead of pretending the flat line is editable.
- `prev` / `next` are the neighbours' handoffs, drawn faint in the gutters so a user can see what this region hands over to.

## GraphKey

```ts
export interface GraphKey { i; x; y; inX; inY; outX; outY; mode }
```

One key and its two tangent handles, already in pixels. `i` is the key's index in the curve, which is what an edit is addressed by, so the component never has to map a dot back to a key itself.

## GraphRamp

```ts
export interface GraphRamp { which; x0; x1; path; keys; editable }
```

One shaded band and the polyline across it. `keys` is `null` unless the easing is ALREADY a `keys(...)` curve - a named curve, a cubic or a spring has no dots of its own to draw. `editable` is the wider question: true when `toKeys` can convert the string at all, which is every curve except a spring (`SpringControls` edits those, and a spring has no keys to give). `useGraphDrag` closes the gap by converting an editable ramp up front, so the dots appear before the first drag rather than after it.

## GraphBand

```ts
export interface GraphBand { x0: number; x1: number; y: number }
```

A horizontal run at one value: the hold between the ramps, and the follow hint that sits under it.

## GraphModel

```ts
export interface GraphModel { width; height; plot; ramps; plateau; hint; ghosts; ticks; yTicks }
```

The whole drawing. `yTicks` is always exactly two entries, in this order: the rest rail then the peak rail. That is not decoration - `progressToY` reads them as the two ends of the value axis, which is what keeps the px-to-progress mapping a property of the model rather than a second formula that could drift from the one the paths were built with.

## GRAPH_W

```ts
export const GRAPH_W = 320;
```

The design's width, the inspector panel's own `--e-panel-w`. The svg scales to whatever width it is given; this is the coordinate system, not a CSS size.

## GRAPH_H

```ts
export const GRAPH_H = 140;
```

Tall enough that a 2x zoom's ramp has slope to read and an overshoot has somewhere to go, short enough to sit in a section next to the timing fields.

## GHOST_PX

```ts
export const GHOST_PX = 16;
```

The gutter on each side of the plot, reserved whether or not a neighbour exists, so the span's own scale never jumps when one appears.

## baseOf

```ts
export const baseOf = (lane: Lane) => number
```

Where the motion rests: scale `1` for a zoom, progress `0` for a layout or a camera move. The ramps run between this and `peak`.

## niceStep

```ts
export function niceStep(spanMs: number): number
```

The tick spacing that leaves four or five ticks across the span, from a fixed ladder of round numbers. A computed "nice" step (the usual `10^floor(log10)` trick) drifts to values like 384 ms on an odd span; a ladder always reads as a time a person would say.

## buildGraph

```ts
export function buildGraph(input: GraphInput, width?: number, height?: number): GraphModel
```

The whole model, in one pass: plot rectangle, ramp windows from the durations, sampled values, the y range with its headroom, the polylines, the hold, the follow hint, the neighbour ghosts, and the ticks.

Two decisions worth naming. The y range is taken from the values actually sampled (plus 8% headroom), never from `[base, peak]`, so a curve that overshoots to 1.3 is drawn overshooting instead of flattening against the top of the box. And the ghosts are sampled over the visible GUTTER rather than over their own duration: the region next door may be seconds long, and all a ghost has to say is which way its curve is heading as it hands over.
