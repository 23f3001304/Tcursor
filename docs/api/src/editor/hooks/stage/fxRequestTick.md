# src/editor/hooks/stage/fxRequestTick.ts

The FX-overlay request half of a composite tick, moved out of `useCompositeLoop.ts` (at the size cap) unchanged when the time remap needed room there. The loop still owns the latches (they are refs it creates) and hands them in; this file builds the cache key and, when the key moved and nothing is in flight, asks the backend for the overlay and latches the response. The cadence cap (`FX_BUCKET_MS`, ~25fps of backend round-trips) lives here now with the key it buckets.

## FxRequestRefs

```ts
export interface FxRequestRefs { fxLastTRef, fxWantRef, fxInflightRef, fxOverlayImgRef, fxSeparableRef, dirtyRef }
```

The loop's latches: the last key actually applied, the key wanted on the most recent tick (so a late response can tell it is stale), whether a request is in flight, the cached image, and whether that image may be faded client-side (`spotAlphaPlan`'s separable path).

## FxRequestTick

```ts
export interface FxRequestTick { frameLayout, fxW, fxH, screenScale, mapFn, cpos, resolvedSpot, plan, cf, clicks, t }
```

Everything one tick already computed that the request needs. `t` is CLIP time: the ripples are keyed to click events on the clip clock; the spotlight arrives already resolved by the caller (on the output clock, against the remapped effects), so no second clock crosses into the request.

`mapFn` is a CANVAS point (0..1 of the recorded frame) to FX-render px - `fxFrameGeometry`'s `mapCanvas`. It is only ever applied to `ClickSample` positions, which are canvas fractions; the cursor point arrives already projected as `cpos`.

## fxRequestTick

```ts
export function fxRequestTick(refs: FxRequestRefs, p: FxRequestTick): void
```

Key = time bucket + cursor cell + spotlight params + (for an unmirrored style) the click list + the FX settings + the camera rect. A resolved response, even a definite "nothing active" `null`, latches the key so the loop stops re-requesting until it changes; only a rejected request stays retryable; a response whose key the loop has already moved past is dropped unlatched.
