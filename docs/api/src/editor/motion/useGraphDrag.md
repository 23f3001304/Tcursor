# src/editor/motion/useGraphDrag.ts

Editing the motion graph. Two halves, deliberately: a set of pure functions that decide where a key or a handle LANDS, taking and returning a `Keys`, and a hook that owns the draft, the selection and the debounced commit. The split is what makes the hard part - clamping, inserting, retiming - testable without mounting anything, which is what `useGraphDrag.test.ts` does for all of it.

**Why every edit goes back out as a string.** The wire form is `easing`, and `keys.ts`'s canonical writer is what both sides agree on, so a drag commits `keys(...)` through the region's existing update op (`update_zoom`, `update_layout_seg`, `update_camera_move`). There is no new field and no second representation of a curve to reconcile.

**One drag, one undo step.** Moves commit through a trailing debounce, the same 80 ms window `Slider.tsx` uses, flushed on pointer release; keyboard edits are discrete and commit at once. The draft input is drawn meanwhile, so the curve tracks the pointer at full rate while the backend sees one op.

## GraphPatch

```ts
export interface GraphPatch { easing?: string; easing_out?: string; inMs?: number; outMs?: number }
```

What a commit asks the inspector to write, in the inspector's own terms. The caller maps the four fields onto its own op: a zoom's `easing` / `easing_out` / `zoom_in_ms` / `zoom_out_ms`, a layout segment's `easing` / `easing_out` / `transition_ms` / `transition_out_ms`, a camera move's `easing` alone. The patch never names a doc field itself, which is why one hook serves three ops.

## GraphSel

```ts
export interface GraphSel { ramp: "in" | "out"; key: number }
```

Which key the keyboard acts on. It is a ramp plus an index rather than an id because a key has no identity in the wire form - it is a position in a list, and an insert or a delete renumbers it.

## useGraphDrag

```ts
export function useGraphDrag(input, onCommit, opts?): { model, selected, handlers, live }
```

The hook. It memoises the model on the input's VALUE (an inspector hands over a fresh object literal every render), keeps the in-flight drag as a draft, and retires that draft when the committed value comes back - the same handoff `CurveEditor` uses, so the curve never blinks between the optimistic draw and the round trip.

`handlers` is three things: `svg` (the focusable canvas - keyboard, the double-click add, and a background press that deselects), `key(which, i)` (a focusable dot with its `aria-label`, whose focus IS the selection, so Tab and the arrow keys work without a second selection model), and `handle(which, i, side)` (a tangent, `aria-hidden` because it has no meaning without sight of the curve; the keyboard shapes a curve by nudging keys).

A pointer gesture captures the ramp window, the curve and the svg's screen rect at press and works from those: a retime drag MOVES the drawing underneath itself, and re-reading the live model mid-gesture would feed the pointer its own output. `opts.readOnly` disables every edit path (a spring, or the read-only graph in Settings); `opts.retimeable` is false for a camera move, whose ramp length is the keyframe spacing and belongs to the timeline, not to this graph.
