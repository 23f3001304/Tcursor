# src/editor/inspectors/curveMath.ts

The geometry behind `CurveEditor`'s canvas: curve space to SVG, pointer to curve space, and the
clamp both share. Pure, so it is testable without a DOM (`curveMath.test.ts`).

This module came back with the curve editor. It went away when the six curve cards were replaced by
a dropdown, and returned when the owner asked where the curve editor had gone: what they had vetoed
was the card grid, not the ability to shape a curve. See `CurveEditor.md`.

The drawn shapes of the six named curves are **not** here. They live in
`src/editor/timeline/curveGlyphs.ts`, which the timeline's transition popover also draws, so the
row, the popover and the canvas cannot show three different pictures of one easing.

## Cubic

```ts
export type Cubic = [number, number, number, number];
```

`[x1, y1, x2, y2]` - the two control points of a CSS-semantics cubic bezier through `(0,0)` and
`(1,1)`, the same four numbers `lib/cubicBezier.ts`'s `cubic(...)` wire form carries.

## VIEW

```ts
export const VIEW: { minX: 0; minY: -30; w: 100; h: 160 }
```

The canvas' SVG box. x `0..100` is progress `0..1`; y `100..0` is value `0..1`; the extra `-30..130`
band is the room a handle has to overshoot in either direction, which a spring-shaped curve needs.

## VIEW_BOX

```ts
export const VIEW_BOX: string
```

`VIEW` as the `viewBox` attribute string.

## Y_MAX

```ts
export const Y_MAX: number
```

## Y_MIN

```ts
export const Y_MIN: number
```

The value bounds the viewBox band implies, and therefore the drag/nudge clamp. Derived from `VIEW`
rather than written twice, and rounded like every other coordinate this module emits so a clamped
handle compares equal to the bound itself.

## CURVE_SEEDS

```ts
export const CURVE_SEEDS: Record<string, Cubic>
```

The cubic each named curve seeds its handles from when the user starts dragging it. Exact for
`linear`, `ease_in`, `ease_out` and `smooth` - `curveMath.test.ts` proves each reproduces `ease(key,
p)` to 4 decimals - and the nearest standard bezier for `ease_in_out`. A spring is not a cubic at
all, so its entry is only the seed a user gets by dragging a spring into a custom curve.

Separate from `curveGlyphs.ts` on purpose: that file publishes what each curve **looks like**, this
one what it **converts to**, and only the editor needs the second.

## toSvg

```ts
export const toSvg: (x: number, y: number) => [number, number]
```

Curve space (progress, value) to SVG user units.

## clampHandle

```ts
export const clampHandle: (x: number, y: number) => [number, number]
```

Clamps a control point to what the canvas can show and the Rust parser accepts: x into `[0,1]`
(which keeps `x(t)` monotonic), y into the viewBox's overshoot band.

## curvePath

```ts
export function curvePath(c: Cubic): string
```

The `d` for a cubic through P0=(0,0), P1, P2, P3=(1,1).

## setHandle

```ts
export function setHandle(c: Cubic, handle: 0 | 1, x: number, y: number): Cubic
```

Replaces one control point (0 = P1, 1 = P2), clamped.

## nudgeHandle

```ts
export const nudgeHandle: (c: Cubic, handle: 0 | 1, dx: number, dy: number, step?: number) => Cubic
```

Keyboard nudge: one arrow press moves a handle by `step` (0.05) in curve space. The handles are
real `role="slider"` controls, so this is how the curve is editable without a pointer.

## clientToCurve

```ts
export function clientToCurve(clientX: number, clientY: number, rect): [number, number]
```

Pointer position to curve space, clamped. `rect` is the canvas element's bounding box, captured once
per drag rather than per move.

## handlePct

```ts
export function handlePct(c: Cubic, handle: 0 | 1): [number, number]
```

Where a handle sits over the canvas as `[left%, top%]`. The dots are HTML laid over the SVG, not
circles inside it, because the canvas draws with `preserveAspectRatio="none"` (a 268x96 box showing
a 100x160 viewBox) and that stretch would squash a circle into an ellipse. This is the exact
inverse of `clientToCurve`, pinned as a round trip in the tests - if the two ever disagreed, a
grabbed handle would jump on its first pointermove.

## curveOf

```ts
export function curveOf(easing: string): Cubic
```

The control points the canvas should show for an easing wire-name: the parsed curve for a custom
`cubic(...)`, else that named preset's seed, else Smooth's (what `valid_easing` coerces an unknown
name to anyway).

## springPathOf

```ts
export function springPathOf(easing: string | undefined | null): string | null
```

The canvas path for an easing that IS a spring (the bare word or a parameterised `spring(k,c,m)`),
sampled at 48 points from the same `spring()` the export evaluates, so the drawing cannot lie about
the oscillator the two sliders under it are tuning. `null` for everything else, which is also how
`CurveEditor` asks "is this a spring" when deciding whether to show handles or sliders.

`curveGlyphs.ts` samples only the ONE default spring, for its static glyph, and keeps its sampler
private; the live one is computed here.
