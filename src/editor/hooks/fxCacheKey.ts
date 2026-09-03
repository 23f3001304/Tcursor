/** Pure seams pulled out of `useCompositeLoop`'s FX-overlay request gating so they're testable
 *  without a canvas/rAF loop. See `useCompositeLoop.md` for how the three fit together: `timeBucket`
 *  caps the request cadence, `fxCacheKey` names "what would this frame's overlay look like", and
 *  `isStaleFxResponse` tells the `.then` handler whether the world moved on while its request was
 *  in flight (only one FX request is ever in flight at a time, so at most one response can go
 *  stale - no counter/token needed, just "does the resolved key still match what's wanted now"). */

/** Round `t` down to the nearest `bucketMs` boundary - caps how often a fresh FX-overlay request
 *  can fire (the spotlight tracks the cursor, which moves on nearly every frame during playback). */
export function timeBucket(t: number, bucketMs: number): number {
  return Math.round(t / bucketMs) * bucketMs;
}

/** The spotlight's share of the cache key: "what would this frame's spotlight look like", built
 *  from the RESOLVED spotlight (region overrides applied), not the raw global settings - otherwise
 *  editing a region's dim/radius/feather/mode in the inspector never changed this string, so the
 *  key never invalidated and the new look only showed up once something else (cursor movement, a
 *  time-bucket change) coincidentally forced a fresh request. That is why it used to need a scrub.
 *
 *  `alpha` is deliberately ABSENT. On the separable path (`spotAlphaPlan`) that is now literally
 *  true: the request is made at a reference alpha of `1` and the live alpha is applied at blit
 *  time, so the cached PNG really is alpha-independent and the fade costs no IPC at all. On the
 *  fallback path the request still carries the live alpha, and leaving it out of the key is what
 *  keeps the key stable across a bucket - putting it in would move `fxWantRef` on nearly every
 *  tick of a fade, so every response would come back "stale" and nothing would ever be applied.
 *  `separable` IS in the key, so flipping paths (a mode or click-style change) re-requests instead
 *  of reusing an image whose alpha basis no longer matches how it would be drawn. */
export function spotParamsKey(
  resolved: { dim: number; radius: number; feather: number; mode: string; tint: [number, number, number] } | null,
  screenScale: number, separable: boolean,
): string {
  if (!resolved) return "off";
  return `${resolved.dim}-${resolved.radius}-${resolved.feather}-${resolved.mode}-${resolved.tint.join(",")}`
    + `-${screenScale.toFixed(3)}-${separable ? "sep" : "live"}`;
}

/** Name the FX-overlay request for one frame's resolved params - used both to skip a redundant
 *  in-flight request (same key = nothing would look different) and, in `.then`, to tell whether
 *  the response that just landed is still wanted. `clicksStr` (sweep-2): the caller passes `""`
 *  for a `stylesMirrored` style (`ripplePreview.ts`) - those clicks never reach this request at
 *  all, so a varying component for them would be dead weight - and the real click list only for
 *  an unmirrored style (Pulse/Glow/Neon/Particles), which still falls back to this overlay and so
 *  still needs its cache key to invalidate when the click list changes. */
export function fxCacheKey(
  tBucket: number, cursorStr: string, spotParamsStr: string,
  clicksStr: string, fxParamsStr: string, camStr: string,
): string {
  return `${tBucket}_${cursorStr}_${spotParamsStr}_${clicksStr}_${fxParamsStr}_${camStr}`;
}

/** Whether an FX-overlay response resolved AFTER the desired key changed out from under it - if
 *  so, the caller should drop it (don't paint it, don't mark it "done") rather than blit an
 *  outdated spotlight/click frame until the next unrelated cache-key change happens to fire a
 *  fresh request. `wantedKey` is whatever the loop most recently computed, independent of
 *  whether a request was actually issued for it this tick. */
export function isStaleFxResponse(requestedKey: string, wantedKey: string): boolean {
  return requestedKey !== wantedKey;
}

/** What the `.then` handler of a RESOLVED (not rejected) FX-overlay request should do. A
 *  rejected promise - an actual IPC/backend failure - never reaches this function; the caller's
 *  `.catch` handles that separately and leaves the key unlatched so it retries next tick. */
export type FxResponseAction =
  | { kind: "stale" }
  | { kind: "apply"; imageUrl: string | null };

/** Decide what a resolved FX-overlay response means. Stale (`isStaleFxResponse`) responses are
 *  dropped outright. Otherwise the response is applied - and a `null` `imageUrl` is just as much
 *  a landed answer as a real one: it means "nothing to draw at this key" (fx disabled, or no
 *  active click/spotlight), a valid terminal state, not a failure. Both cases latch the same way
 *  in the caller, which is what stops the loop from re-requesting a key it already knows renders
 *  nothing - the common no-fx case - every tick forever. */
export function fxResponseAction(requestedKey: string, wantedKey: string, imageUrl: string | null): FxResponseAction {
  if (isStaleFxResponse(requestedKey, wantedKey)) return { kind: "stale" };
  return { kind: "apply", imageUrl };
}
