# src/editor/hooks/fxCacheKey.ts

Pure seams pulled out of `useCompositeLoop`'s FX-overlay request gating so they're unit-testable without a canvas/rAF loop: bucketing time, naming a frame's resolved FX params, detecting a stale response, and deciding what a resolved (non-rejected) response means for the loop's latch state.

## timeBucket

```ts
export function timeBucket(t: number, bucketMs: number): number
```

Rounds `t` to the nearest `bucketMs` boundary (`Math.round(t / bucketMs) * bucketMs`) - caps how often a fresh FX-overlay request can fire, since the spotlight tracks the cursor and would otherwise invalidate the cache at full 60fps during playback.

## fxCacheKey

```ts
export function fxCacheKey(
  tBucket: number, cursorStr: string, spotParamsStr: string,
  clicksStr: string, fxParamsStr: string, camStr: string,
): string
```

Joins the bucketed time and every resolved-FX component into one string key. Used both to skip a redundant in-flight request (same key = nothing would look different) and, in `useCompositeLoop`'s response handler, as the value compared against `fxWantRef.current` to detect staleness.

`clicksStr` (sweep-2): the caller passes `""` for a `stylesMirrored` style (`ripplePreview.ts`) - those clicks never reach the FX-overlay request at all (see `fxOverlay.ts`), so a varying component for them would be dead weight - and the real click-list string only for an unmirrored style (Pulse/Glow/Neon/Particles), which still falls back to the overlay and so still needs its cache key to invalidate when the click list changes (`overlayNeedsClicks`, `ripplePreview.ts`).

### Used by

`useCompositeLoop` (`src/editor/hooks/useCompositeLoop.ts`) - builds the key each tick from the zoom-projected cursor position, the resolved spotlight params, the (conditional) click list, the click-FX settings, and the camera-exclusion rect.

## isStaleFxResponse

```ts
export function isStaleFxResponse(requestedKey: string, wantedKey: string): boolean
```

Whether an FX-overlay response resolved AFTER the desired key changed out from under it. Only one FX request is ever in flight at a time (`useCompositeLoop`'s `fxInflightRef` gate), so at most one response can go stale - no monotonic counter/token is needed, just "does the resolved key still match what's wanted now" (`requestedKey !== wantedKey`).

### Used by

`useCompositeLoop` (indirectly, via `fxResponseAction`) - `requestedKey` is the cache key closed over at request time; `wantedKey` is `fxWantRef.current`, updated every tick regardless of whether a request fired. A stale response is dropped (never applied, never latched into `fxLastTRef`) so a doc/effects change mid-request cannot leave an outdated spotlight/click frame blitted until some unrelated later change happens to fire a fresh request.

## FxResponseAction

```ts
export type FxResponseAction =
  | { kind: "stale" }
  | { kind: "apply"; imageUrl: string | null };
```

What a RESOLVED (not rejected) FX-overlay response means for the loop's latch state. A rejected promise - an actual IPC/backend failure - never produces one of these; `useCompositeLoop`'s own `.catch` handles that separately and leaves the key unlatched so it retries next tick.

## fxResponseAction

```ts
export function fxResponseAction(requestedKey: string, wantedKey: string, imageUrl: string | null): FxResponseAction
```

Decides what to do with a resolved FX-overlay response. Stale (`isStaleFxResponse`) responses are dropped outright (`{ kind: "stale" }`). Otherwise the response is applied (`{ kind: "apply", imageUrl }`) - and a `null` `imageUrl` is just as much a landed answer as a real one: it means "nothing to draw at this key" (fx disabled, or no active click/spotlight), a valid terminal state, not a failure. Both cases latch the same way in the caller, which is what stops the composite loop from re-requesting a key it already knows renders nothing - the common no-fx case - every tick forever (the bug this function exists to make impossible-to-regress).

### Used by

`useCompositeLoop` - called from the FX-overlay request's `.then` with the cache key that was requested, `fxWantRef.current` (what's wanted now), and the resolved `url`. On `"apply"`, latches `fxLastTRef` to the requested key regardless of whether `imageUrl` is `null` or a string; on `"stale"`, does nothing (leaves the key unlatched).
