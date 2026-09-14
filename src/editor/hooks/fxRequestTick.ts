import type { RefObject } from "react";
import type { ClickSample, PreviewLayout } from "../../lib/ipc";
import type { ClickFxSettings } from "../../hud/settings/settings";
import { fxCamRect } from "../stage/fxGeometry";
import { requestFxOverlay, type FxCamRect } from "../stage/fxOverlay";
import { overlayNeedsClicks } from "../stage/ripplePreview";
import type { resolveSpotlight, spotAlphaPlan } from "../stage/spotlightPreview";
import { fxCacheKey, fxResponseAction, spotParamsKey, timeBucket } from "./fxCacheKey";

const FX_BUCKET_MS = 40; // ~25fps cap on backend FX-overlay requests (see useCompositeLoop's banner)

/** The loop's FX-overlay latches: the last key actually applied, the key wanted on the most recent
 *  tick, whether a request is in flight, the cached image and whether it may be faded client-side. */
export interface FxRequestRefs {
  fxLastTRef: RefObject<string>; fxWantRef: RefObject<string>; fxInflightRef: RefObject<boolean>;
  fxOverlayImgRef: RefObject<HTMLImageElement | null>; fxSeparableRef: RefObject<boolean>; dirtyRef: RefObject<boolean>;
}

/** Everything one tick already computed that the request needs. `t` is CLIP time: the ripples are
 *  keyed to click events on the clip clock; the spotlight arrives already resolved (on the output
 *  clock, by the caller), so no second clock crosses into the request. */
export interface FxRequestTick {
  frameLayout: PreviewLayout | null; fxW: number; fxH: number; screenScale: number;
  /** CANVAS point (0..1 of the recorded frame) -> FX-render px - `fxFrameGeometry`'s `mapCanvas`.
   *  It is only ever applied to `ClickSample` positions, which are canvas fractions; the cursor
   *  point arrives already projected as `cpos`. */
  mapFn: (fx: number, fy: number) => [number, number] | null; cpos: [number, number] | null;
  resolvedSpot: ReturnType<typeof resolveSpotlight>; plan: ReturnType<typeof spotAlphaPlan>;
  cf: ClickFxSettings; clicks: ClickSample[]; t: number;
}

/** The FX-overlay request half of a composite tick, moved out of `useCompositeLoop.ts` (at the
 *  size cap) unchanged: build the cache key, and when it moved and nothing is in flight, ask the
 *  backend for the overlay at this key and latch the response. */
export function fxRequestTick(refs: FxRequestRefs, p: FxRequestTick): void {
  const { frameLayout, fxW, fxH, screenScale, mapFn, cpos, resolvedSpot, plan, cf, clicks, t } = p;
  const camRect: FxCamRect = fxCamRect(frameLayout, fxW, fxH);
  const spotParamsStr = spotParamsKey(resolvedSpot, screenScale, plan.separable);
  const cursorStr = cpos ? `${Math.round(cpos[0])}-${Math.round(cpos[1])}` : "none";
  // "" for a mirrored style (ripplePreview.ts draws it instead) - the real click list
  // only for an unmirrored one, which still falls back to this overlay (see fxCacheKey.ts).
  const clicksStr = overlayNeedsClicks(cf.style) ? clicks.map(clk => `${clk.t}-${clk.x}-${clk.y}`).join(";") : "";
  const fxParamsStr = `${cf.style}-${cf.color.join(",")}-${cf.intensity}-${cf.enabled}-${cf.spotlight_dim_camera}`;
  const camStr = camRect ? camRect.rect.map(v => Math.round(v)).join(",") + `-${Math.round(camRect.radius)}` : "none";
  // cursorStr alone would invalidate the cache at full 60fps during playback - timeBucket
  // caps how often that's allowed to trigger a backend round-trip.
  const cacheKey = fxCacheKey(timeBucket(t, FX_BUCKET_MS), cursorStr, spotParamsStr, clicksStr, fxParamsStr, camStr);
  refs.fxWantRef.current = cacheKey; // every tick, fired or not - lets `.then` detect staleness below

  if (!refs.fxInflightRef.current && cacheKey !== refs.fxLastTRef.current) {
    refs.fxInflightRef.current = true;
    // Reuses the tick's ONE resolveSpotlight call - resolving again would double-advance the sim.
    // `requestAlpha` is 1 on the separable path (a reference, alpha-independent overlay)
    // and the live alpha otherwise; `separable` is captured so the response latches the
    // basis its own pixels were rendered at, not whatever the plan says by then.
    const separable = plan.separable;
    const reqSpot = resolvedSpot ? { ...resolvedSpot, alpha: plan.requestAlpha } : null;
    requestFxOverlay(fxW, fxH, clicks, t, cpos, reqSpot, cf, mapFn, screenScale, camRect)
      .then(url => {
        refs.fxInflightRef.current = false;
        const action = fxResponseAction(cacheKey, refs.fxWantRef.current, url);
        // Stale (wanted key moved on): drop it unlatched, next tick reissues for real.
        if (action.kind === "stale") return;
        // Any RESOLVED response - even a definite "off" -> `null`, the common no-fx
        // case - is a valid terminal state: latch so the loop stops re-requesting until
        // the key changes. Only a REJECTED request (`.catch` below) stays retryable.
        refs.fxLastTRef.current = cacheKey;
        if (action.imageUrl) {
          const img = new Image();
          img.onload = () => { refs.fxOverlayImgRef.current = img; refs.fxSeparableRef.current = separable; refs.dirtyRef.current = true; };
          img.src = action.imageUrl;
        } else {
          refs.fxOverlayImgRef.current = null; refs.fxSeparableRef.current = false; refs.dirtyRef.current = true; // nothing active at this key
        }
      })
      .catch(() => { refs.fxInflightRef.current = false; }); // failure: unlatched, retried
  }
}
