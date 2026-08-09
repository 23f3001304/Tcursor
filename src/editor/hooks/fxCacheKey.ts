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

/** Name the FX-overlay request for one frame's resolved params - used both to skip a redundant
 *  in-flight request (same key = nothing would look different) and, in `.then`, to tell whether
 *  the response that just landed is still wanted. */
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
