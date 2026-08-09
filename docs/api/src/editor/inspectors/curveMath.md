# src/editor/inspectors/curveMath.ts

The pure geometry behind `CurveEditor`: curve-space <-> SVG mapping, the drag/nudge clamp, path rendering, and the "which control points should the editor show for this easing string" lookup. No React, no state - all of it is unit-tested in `curveMath.test.ts`.

## VIEW

```ts
export const VIEW = { minX: 0, minY: -30, w: 100, h: 160 } as const;
export const VIEW_BOX: string;
```

The curve cards' shared SVG viewBox, and its string form. `x 0..100` is progress `0..1`; `y 100..0` is value `0..1`; the extra `-30..130` band is the room a handle has to overshoot in either direction. Every other constant here is derived from it, so widening the band is a one-line change.

## Y_MAX

```ts
export const Y_MAX: number; // 1.3
```

The upper value bound the viewBox band implies - how far a handle may overshoot past 1. Rounded to 3 decimals like every coordinate this module emits, so a clamped handle compares exactly equal to the bound rather than to a float-noise neighbour.

## Y_MIN

```ts
export const Y_MIN: number; // -0.3
```

The lower bound, the same derivation mirrored: how far below 0 a handle may dip.

## toSvg

```ts
export const toSvg: (x: number, y: number) => [number, number]
```

Curve space -> SVG user units. `(0,0)` is the bottom-left corner (`[0,100]`), `(1,1)` the top-right (`[100,0]`).

## clampHandle

```ts
export const clampHandle: (x: number, y: number) => [number, number]
```

Clamp a control point to what the editor can show AND what the Rust parser accepts: `x` into `[0,1]` (which is what keeps `x(t)` monotonic), `y` into the viewBox's overshoot band. Rounds both to 3 decimals.

## curvePath

```ts
export function curvePath(c: Cubic): string
```

The SVG `d` for a cubic through P0=(0,0), P1, P2, P3=(1,1). Used both for the expanded editor's live curve and for the Custom card's glyph.

## setHandle

```ts
export function setHandle(c: Cubic, handle: 0 | 1, x: number, y: number): Cubic
```

Replace one control point (`0` = P1, `1` = P2), clamped. Returns a NEW tuple - the caller's copy is never mutated, which is what lets `CurveEditor` hold a pre-drag base and derive from it on every pointer move.

## nudgeHandle

```ts
export const nudgeHandle: (c: Cubic, handle: 0 | 1, dx: number, dy: number, step?: number) => Cubic
```

The keyboard form of `setHandle`: one arrow press moves a handle by `step` (default `0.05`) in curve space, with the same clamp and the same no-mutation guarantee.

## clientToCurve

```ts
export function clientToCurve(clientX: number, clientY: number,
  rect: { left: number; top: number; width: number; height: number }): [number, number]
```

Pointer position -> curve space, clamped. `rect` is the editor SVG's bounding box, captured once at pointer-down. Derived from `VIEW` rather than hardcoded, and pinned as the exact inverse of `toSvg` by a round-trip test (including a `y = 1.2` overshoot point).

## curveOf

```ts
export function curveOf(easing: string): Cubic
```

The control points the editor should show for an easing wire-name:

1. a custom `cubic(...)` parses straight back to its own handles;
2. a named preset seeds from that preset's `c` (`curves.ts`);
3. anything unknown seeds from Smooth - which is also what `valid_easing` would coerce it to, so the editor and the backend agree on what an unrecognised name means.

Returns a copy, never the shared `CAM_CURVES` entry, so editing cannot mutate the preset table.
