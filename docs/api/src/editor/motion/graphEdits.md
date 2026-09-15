# src/editor/motion/graphEdits.ts

Pure curve edits for the motion graph: moving a key or its handle, adding and removing keys, keyboard nudges, the retime rule for a ramp's own end key, and the `GraphPatch` an edit commits. Split out of `useGraphDrag.ts`, which keeps the React hook that owns the draft, the debounce and the pointer/keyboard handlers; every function here takes `Keys` (or a `GraphInput`) and returns a new one, so `useGraphDrag.test.ts` can pin the editing rules with no DOM.

## GraphPatch

```ts
export interface GraphPatch {
  easing?: string;
  easing_out?: string;
  inMs?: number;
  outMs?: number;
}
```

What one edit commits upward: the in-ramp's curve and duration, the out-ramp's, or both. Every field is optional because an edit only ever touches the ramp it was made on - the inspector spreads the patch into its own `update_*` op, so an absent field leaves the doc alone.

## MIN_RAMP_MS

```ts
export const MIN_RAMP_MS = 0;
```

The floor a retime drag clamps a duration to, matching what `ZoomInspector`'s In and Out cells already clamp `zoom_in_ms` to. Zero is a legal, meaningful value: a hard cut. A ramp dragged to zero leaves no band to grab, and the way back is the Timing field that set the same number.

## NUDGE

```ts
export const NUDGE = 0.01;
```

One arrow press, as a fraction of the ramp. At 256 px of plot that is about two and a half pixels: visible, and fine enough to settle a curve that a pointer can only get close to.

## NUDGE_BIG

```ts
export const NUDGE_BIG = 0.1;
```

The same press with Shift, a tenth of the ramp - the coarse step for getting somewhere, where `NUDGE` is the one for arriving.

## toKeysInput

```ts
export function toKeysInput(input: GraphInput): GraphInput
```

The input with every convertible ramp already rewritten in keys form. A region that still carries `"smooth"` or a `cubic(...)` therefore shows its dots BEFORE the first drag instead of after it, which is the difference between a graph you can edit and a graph you have to discover you can edit.

The conversion is local: nothing is written until a drag actually commits, so opening an inspector never touches the doc. A spring converts to nothing (`toKeys` returns `null`) and is left exactly as it is, which also leaves its ramp `editable: false` and its dots undrawn. The one inexactness is `ease_in_out`, whose piecewise quadratic has no exact cubic; `toKeys` uses the fitted handle length, within 5.2e-3 of a ramp (`keys.md`, `EASE_IN_OUT_H`).

## moveKey

```ts
export function moveKey(k: Keys, i: number, t: number, v: number): Keys
```

Move one key. The first and the last are PINNED in time: the wire form demands `t` exactly 0 and 1, and a curve that lost either would not parse at all. An inner key is held strictly between its neighbours, one 3-decimal tick clear of each, so time never folds over inside a segment and the evaluator's bisection always has exactly one root. `v` is left completely free - below 0 is anticipation, above 1 is overshoot, and both are the point.

## moveHandle

```ts
export function moveHandle(k: Keys, i: number, side: "in" | "out", dx: number, dy: number): Keys
```

Move one tangent handle. The clamp is `canonicalKeys`' own (`out_dx` into `[0, next.t - t]`, `in_dx` into `[-(t - prev.t), 0]`), not a copy of it, so the editor and the writer can never disagree about what a legal handle is. `dy` is never clamped.

## addKeyAt

```ts
export function addKeyAt(k: Keys, p: number): Keys
```

Add a key at ramp progress `p`, sitting exactly ON the curve, so a double-click adds a control point without changing the shape it was added to. It inherits the mode of the segment it splits, so splitting a hold gives two holds rather than quietly turning the segment into a bezier.

Refused - by returning the SAME object, which is how the hook tells "nothing happened" from "an edit" - when the curve already has `MAX_KEYS` keys or when `p` lands on a key that is already there.

## removeKey

```ts
export function removeKey(k: Keys, i: number): Keys
```

Delete an inner key. The endpoints are refused: they are the ramp's own ends, and without them the string is not a parseable `keys(...)`. Same-object-back means refused.

## nudgeKey

```ts
export function nudgeKey(k: Keys, i: number, dt: number, dv: number, big?: boolean): Keys
```

One arrow press, through `moveKey`, so a nudge obeys the same pins and clamps a drag does. Nudging an endpoint therefore moves its value only, which is exactly right: that is how you give a ramp an overshoot without touching its timing.

## retimeIndex

```ts
export const retimeIndex = (which: "in" | "out", n: number) => number
```

Which key's horizontal drag retimes the ramp: the in ramp's LAST and the out ramp's FIRST. Both are the same boundary seen from either side - the edge where the ramp meets the hold - so the gesture is "drag the end of the ramp" in both cases, and the region's own start and end stay where the timeline put them.

## retimeMs

```ts
export function retimeMs(input: GraphInput, which: "in" | "out", ms: number): number
```

The duration a retime drag writes: whole milliseconds (the doc's fields are `u32`), never below `MIN_RAMP_MS`, never longer than the region itself. The upper clamp is not a correctness fix - the export refits a pair that together outlast their span (`fitDurations`) - it just keeps the drag from writing a number the render would silently ignore.

## withCurve

```ts
export function withCurve(input: GraphInput, which: "in" | "out", k: Keys, durMs?: number): GraphInput
```

The edit as a draft input: the same curve the commit carries, in the shape `buildGraph` draws. One function so the picture under the pointer and the string on the wire can never be two different edits.

## patchOf

```ts
export const patchOf = (which: "in" | "out", k: Keys, durMs?: number): GraphPatch
```

The edit as a patch. The in ramp writes `easing` (and `inMs` when the drag retimed it), the out ramp `easing_out` and `outMs`. A zoom that had no `easing_out` gets one the first time its out ramp is touched, which is exactly the "absent means the same curve as `easing`" rule the schema addition defines.
