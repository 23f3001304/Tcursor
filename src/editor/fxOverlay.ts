import type { ClickSample } from "../lib/ipc";
import { previewFxOverlay, type FxOverlayParams } from "../lib/ipc";
import type { ClickFxSettings } from "../hud/settings";
import { resolveSpotlight, type SpotlightInput, type SpotlightSimState } from "./spotlightPreview";

const RIPPLE_MS = 500;

/** Build FxOverlayParams from the current preview state and call the backend.
 *  Returns a data URL PNG or null if nothing is active. The backend runs the
 *  exact same GPU/CPU shader pipeline the export uses. */
export async function requestFxOverlay(
  ow: number, oh: number,
  clicks: ClickSample[], now: number,
  cursorPx: [number, number] | null,
  spotlight: SpotlightInput | null,
  clickfx: ClickFxSettings,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  sim: SpotlightSimState,
  // Screen-panel height as a fraction of the FX render height (= layout.screen[3]). The
  // radius/feather settings are fractions of the SCREEN, so pre-scale by this before the
  // backend's `oh * frac` - mirrors the export's `fx_state_at` (scene.screen.h / oh) so the
  // spotlight tracks the screen panel in every layout instead of the whole frame.
  screenScale: number,
): Promise<string | null> {
  // Build active click hits in output pixels
  const hits: [number, number, number][] = [];
  if (clickfx.style !== "none" && clickfx.enabled) {
    for (const c of clicks) {
      const dt = now - c.t;
      if (dt < 0 || dt > RIPPLE_MS) continue;
      const p = mapFn(c.x, c.y);
      if (!p) continue;
      hits.push([p[0], p[1], dt / RIPPLE_MS]);
    }
  }

  // Resolve spotlight
  const resolved = spotlight ? resolveSpotlight(spotlight, now, sim) : null;
  const spotActive = resolved && resolved.alpha > 0.001 && cursorPx;

  // Skip if nothing to render
  if (hits.length === 0 && !spotActive) return null;

  const params: FxOverlayParams = {
    ow, oh,
    style: clickfx.style, color: clickfx.color, intensity: clickfx.intensity,
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
  }

  try {
    return await previewFxOverlay(params);
  } catch (e) {
    if (import.meta.env.DEV) console.warn("preview_fx_overlay:", e);
    return null;
  }
}
