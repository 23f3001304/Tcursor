import type { ClickSample } from "../../lib/ipc";
import { previewFxOverlay, type FxOverlayParams } from "../../lib/ipc";
import type { ClickFxSettings } from "../../hud/settings/settings";
import type { ResolvedSpotlight } from "./spotlightPreview";
import { activeRippleHits, overlayNeedsClicks } from "./ripplePreview";

/** Camera PiP rect in FX-render px (min_x, min_y, max_x, max_y) + corner radius, or null when
 *  no camera panel is active this frame - mirrors the export's Spot.cam_rect/cam_radius. */
export type FxCamRect = { rect: [number, number, number, number]; radius: number } | null;

/** Build FxOverlayParams from the current preview state and call the backend. Returns a data URL
 *  PNG or null if nothing is active. The backend runs the exact same GPU/CPU shader pipeline the
 *  export uses - for spotlight + video-fx always, and for CLICK RIPPLES too when the active style
 *  isn't one `ripplePreview.ts` draws client-side (sweep-2: `stylesMirrored`'s two styles - Ripple
 *  the default, and Shockwave - used to ALSO go through this request, at its `FX_BUCKET_MS`
 *  cadence + one IPC round-trip per update, which read as laggy/stuck during playback; they now
 *  draw straight on the preview canvas every tick instead). `overlayNeedsClicks` gates which case
 *  this is: the other four click-fx styles (Pulse/Glow/Neon/Particles) are NOT mirrored yet, so
 *  they keep going through this exact path as before - laggy, but still visible in the live
 *  preview, rather than a silent regression to nothing. The export is unaffected either way (it
 *  never goes through this overlay path at all).
 *
 *  `resolved` must already be the caller's own `resolveSpotlight(...)` result for this exact
 *  frame, not raw settings to resolve here: `resolveSpotlight` mutates a `SpotlightSimState` in
 *  place (region handoff/transition tracking), so it must run exactly once per composite tick.
 *  The caller also uses that same resolved value to build the FX request's cache key, so passing
 *  it in here (instead of re-resolving) keeps "what the cache key says the spotlight looks like"
 *  and "what actually gets requested" in sync by construction - they're the same call. */
export async function requestFxOverlay(
  ow: number, oh: number,
  clicks: ClickSample[], now: number,
  cursorPx: [number, number] | null,
  resolved: ResolvedSpotlight | null,
  clickfx: ClickFxSettings,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  // Screen-panel height as a fraction of the FX render height (= layout.screen[3]). The
  // radius/feather settings are fractions of the SCREEN, so pre-scale by this before the
  // backend's `oh * frac` - mirrors the export's `fx_state_at` (scene.screen.h / oh) so the
  // spotlight tracks the screen panel in every layout instead of the whole frame.
  screenScale: number,
  // The active camera panel's rect this frame (FX-render px), or null when no camera panel is
  // shown - threaded to the backend so it can undo the spotlight dim inside it when the
  // "don't dim the webcam" setting (clickfx.spotlight_dim_camera) is off.
  camRect: FxCamRect,
): Promise<string | null> {
  // `enabled` is the master switch for ALL fx, spotlight included - the export returns before
  // drawing anything when it is off (`fx_state.rs`'s `render`). Without this the preview kept
  // showing the spotlight for a recording the export renders with no fx at all.
  if (!clickfx.enabled) return null;

  // Only build hits for a style ripplePreview.ts does NOT already draw client-side - see the
  // module doc and `overlayNeedsClicks`. `activeRippleHits` is the SAME lifetime/boundary math
  // ripplePreview.ts uses, so a Pulse/Glow/Neon/Particles ring here is exactly as long-lived as a
  // mirrored Ripple/Shockwave one would be.
  const hits: [number, number, number][] = [];
  if (overlayNeedsClicks(clickfx.style)) {
    for (const h of activeRippleHits(clicks, now)) {
      const p = mapFn(h.x, h.y);
      if (p) hits.push([p[0], p[1], h.progress]);
    }
  }

  const spotActive = resolved && resolved.alpha > 0.001 && cursorPx;

  // Skip if nothing to render.
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
    if (camRect) {
      params.camRect = camRect.rect;
      params.camRadius = camRect.radius;
      params.dimCamera = clickfx.spotlight_dim_camera;
    }
  }

  try {
    return await previewFxOverlay(params);
  } catch (e) {
    // Rethrow rather than swallow to null: a `null` RETURN (above) means "nothing to draw here",
    // a valid, latch-able terminal state for this cache key. A rejected IPC call is a genuine
    // failure and must stay a rejected promise so the caller's `.catch` keeps it retryable
    // instead of also latching it as "done".
    if (import.meta.env.DEV) console.warn("preview_fx_overlay:", e);
    throw e;
  }
}
