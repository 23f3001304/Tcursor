# src/editor/hooks/fxCacheKey.ts

Pure seams pulled out of `useCompositeLoop`'s FX-overlay request gating so they're unit-testable without a canvas/rAF loop: bucketing time, naming a frame's resolved FX params, and detecting a stale response.

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

### Used by

`useCompositeLoop` (`src/editor/hooks/useCompositeLoop.ts`) - builds the key each tick from the zoom-projected cursor position, the resolved spotlight params, the click track, the click-FX settings, and the camera-exclusion rect.

## isStaleFxResponse

```ts
export function isStaleFxResponse(requestedKey: string, wantedKey: string): boolean
```

Whether an FX-overlay response resolved AFTER the desired key changed out from under it. Only one FX request is ever in flight at a time (`useCompositeLoop`'s `fxInflightRef` gate), so at most one response can go stale - no monotonic counter/token is needed, just "does the resolved key still match what's wanted now" (`requestedKey !== wantedKey`).

### Used by

`useCompositeLoop` - `requestedKey` is the cache key closed over at request time; `wantedKey` is `fxWantRef.current`, updated every tick regardless of whether a request fired. A stale response is dropped (never applied, never latched into `fxLastTRef`) so a doc/effects change mid-request cannot leave an outdated spotlight/click frame blitted until some unrelated later change happens to fire a fresh request.
