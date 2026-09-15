import type { ClickSample } from "../../../shared/ipc";
import { previewFxOverlay, type FxOverlayParams } from "../../../shared/ipc";
import type { ClickFxSettings } from "../../../hud/settings/settings";
import type { ResolvedSpotlight } from "./spotlightPreview";
import { activeRippleHits, overlayNeedsClicks } from "./ripplePreview";

export type FxCamRect = { rect: [number, number, number, number]; radius: number } | null;

export async function requestFxOverlay(
  ow: number,
  oh: number,
  clicks: ClickSample[],
  now: number,
  cursorPx: [number, number] | null,
  resolved: ResolvedSpotlight | null,
  clickfx: ClickFxSettings,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  screenScale: number,
  camRect: FxCamRect,
): Promise<string | null> {
  if (!clickfx.enabled) return null;

  const hits: [number, number, number][] = [];
  if (overlayNeedsClicks(clickfx.style)) {
    for (const h of activeRippleHits(clicks, now)) {
      const p = mapFn(h.x, h.y);
      if (p) hits.push([p[0], p[1], h.progress]);
    }
  }

  const spotActive = resolved && resolved.alpha > 0.001 && cursorPx;

  if (hits.length === 0 && !spotActive) return null;

  const params: FxOverlayParams = {
    ow,
    oh,
    style: clickfx.style,
    color: clickfx.color,
    intensity: clickfx.intensity,
    hits,
  };

  if (spotActive && cursorPx) {
    params.spotCx = cursorPx[0];
    params.spotCy = cursorPx[1];
    params.spotDim = resolved!.dim;
    params.spotRadius = resolved!.radius * screenScale;
    params.spotFeather = resolved!.feather * screenScale;
    params.spotAlpha = resolved!.alpha;
    params.spotMode = resolved!.mode;
    params.spotTint = resolved!.tint;
    params.spotT = now / 1000.0;
    if (camRect) {
      params.camRect = camRect.rect;
      params.camRadius = camRect.radius;
      params.dimCamera = clickfx.spotlight_dim_camera;
    }
  }

  try {
    return await previewFxOverlay(params);
  } catch (e) {
    if (import.meta.env.DEV) console.warn("preview_fx_overlay:", e);
    throw e;
  }
}
