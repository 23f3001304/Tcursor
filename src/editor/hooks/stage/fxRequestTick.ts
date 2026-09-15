import type { RefObject } from "react";
import type { ClickSample, PreviewLayout } from "../../../shared/ipc";
import type { ClickFxSettings } from "../../../hud/settings/settings";
import { fxCamRect } from "../../stage/fx/fxGeometry";
import { requestFxOverlay, type FxCamRect } from "../../stage/fx/fxOverlay";
import { overlayNeedsClicks } from "../../stage/fx/ripplePreview";
import type { resolveSpotlight, spotAlphaPlan } from "../../stage/fx/spotlightPreview";
import { fxCacheKey, fxResponseAction, spotParamsKey, timeBucket } from "./fxCacheKey";

const FX_BUCKET_MS = 40;

export interface FxRequestRefs {
  fxLastTRef: RefObject<string>;
  fxWantRef: RefObject<string>;
  fxInflightRef: RefObject<boolean>;
  fxOverlayImgRef: RefObject<HTMLImageElement | null>;
  fxSeparableRef: RefObject<boolean>;
  dirtyRef: RefObject<boolean>;
}

export interface FxRequestTick {
  frameLayout: PreviewLayout | null;
  fxW: number;
  fxH: number;
  screenScale: number;
  mapFn: (fx: number, fy: number) => [number, number] | null;
  cpos: [number, number] | null;
  resolvedSpot: ReturnType<typeof resolveSpotlight>;
  plan: ReturnType<typeof spotAlphaPlan>;
  cf: ClickFxSettings;
  clicks: ClickSample[];
  t: number;
}

export function fxRequestTick(refs: FxRequestRefs, p: FxRequestTick): void {
  const { frameLayout, fxW, fxH, screenScale, mapFn, cpos, resolvedSpot, plan, cf, clicks, t } = p;
  const camRect: FxCamRect = fxCamRect(frameLayout, fxW, fxH);
  const spotParamsStr = spotParamsKey(resolvedSpot, screenScale, plan.separable);
  const cursorStr = cpos ? `${Math.round(cpos[0])}-${Math.round(cpos[1])}` : "none";
  const clicksStr = overlayNeedsClicks(cf.style)
    ? clicks.map((clk) => `${clk.t}-${clk.x}-${clk.y}`).join(";")
    : "";
  const fxParamsStr = `${cf.style}-${cf.color.join(",")}-${cf.intensity}-${cf.enabled}-${cf.spotlight_dim_camera}`;
  const camStr = camRect
    ? camRect.rect.map((v) => Math.round(v)).join(",") + `-${Math.round(camRect.radius)}`
    : "none";
  const cacheKey = fxCacheKey(
    timeBucket(t, FX_BUCKET_MS),
    cursorStr,
    spotParamsStr,
    clicksStr,
    fxParamsStr,
    camStr,
  );
  refs.fxWantRef.current = cacheKey;

  if (!refs.fxInflightRef.current && cacheKey !== refs.fxLastTRef.current) {
    refs.fxInflightRef.current = true;
    const separable = plan.separable;
    const reqSpot = resolvedSpot ? { ...resolvedSpot, alpha: plan.requestAlpha } : null;
    requestFxOverlay(fxW, fxH, clicks, t, cpos, reqSpot, cf, mapFn, screenScale, camRect)
      .then((url) => {
        refs.fxInflightRef.current = false;
        const action = fxResponseAction(cacheKey, refs.fxWantRef.current, url);
        if (action.kind === "stale") return;
        refs.fxLastTRef.current = cacheKey;
        if (action.imageUrl) {
          const img = new Image();
          img.onload = () => {
            refs.fxOverlayImgRef.current = img;
            refs.fxSeparableRef.current = separable;
            refs.dirtyRef.current = true;
          };
          img.src = action.imageUrl;
        } else {
          refs.fxOverlayImgRef.current = null;
          refs.fxSeparableRef.current = false;
          refs.dirtyRef.current = true;
        }
      })
      .catch(() => {
        refs.fxInflightRef.current = false;
      });
  }
}
