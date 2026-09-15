# src/editor/director/targets.ts

Live DOM anchoring for the AI director's fake pointer (`DirectorPointer.tsx`) - pure measurement functions, no state, no side effects, so they're unit-testable with stub `getBoundingClientRect` rects (`targets.test.ts`). Every caller re-queries the DOM and re-measures on EACH step (`useAiRun.ts`'s replay loop calls `document.querySelector(".e-tlbody")` fresh every iteration, nothing here is cached across a run) - so a window resize mid-run never leaves the pointer aiming at a stale rect.

## Lane

```ts
export type Lane = "zoom" | "fx" | "trim-in" | "trim-out"
```

Which timeline lane a target point is on - drives both the x-axis semantics (always an ms/dur fraction of the track) and, for `"zoom"` and `"fx"`, which row `timelinePointForMs` centers on.

## timelinePointForMs

```ts
export function timelinePointForMs(trackEl: HTMLElement, ms: number, dur: number, lane: Lane): { x: number; y: number }
```

Where the fake pointer should aim to act at `ms` on `lane`, measured against the timeline track element (`.e-tlbody`) right now.

### Behavior

`x` is `trackEl`'s rect left + a pure `ms/dur` fraction of its width, clamped to `[0, 1]` first (so an out-of-range `ms`, e.g. a trim target past the clip end, lands on the track's own edge rather than off it) and `0` when `dur <= 0` (no division by zero). `y` is the vertical center of the lane's row when one exists (`ROW_SELECTOR` maps `"zoom"` to `.e-zoomrow` and `"fx"` to `.e-fxrow`, the two tracks that stack into layered rows), else the track's own vertical center - `"trim-in"`/`"trim-out"` always fall through to the track center, since the trim handles have no row of their own.

For the zoom and effects lanes, the target is the BOTTOM-most row - the LAST row in DOM order. `Timeline.tsx` renders the highest layer first and layer 0 last, and a freshly-added zoom or effect always lands on layer 0, so that row is where the pill the replay is pointing at actually sits.

## rectCenter

```ts
export function rectCenter(el: Element): { x: number; y: number }
```

The midpoint of any element's current `getBoundingClientRect()`.

## anchorPoint

```ts
export function anchorPoint(name: string): { x: number; y: number } | null
```

The center of the currently-mounted `data-director-anchor="{name}"` element. The Transport wand button and the AiPanel run button both carry `data-director-anchor="wand"`; when both are mounted at once (the AI panel is the open tab), `document.querySelector` returns the FIRST match in document order, and the AiPanel button sits earlier in the DOM than `Transport` - so the run visibly starts from whichever one the user can actually see. Returns `null` when no matching anchor is mounted.
