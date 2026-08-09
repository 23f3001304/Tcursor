import type { CamZoomAction, Zoom } from "../../lib/edit";
import type { ZoomSettings } from "../../hud/settings/settings";

/** `PreviewLayout.cam`: [x, y, w, h, radius, ringPx, ringR, ringG, ringB], all fractions of the
 *  preview canvas (per-axis: x/w over width, y/h over height). */
export type CamTuple = [number, number, number, number, number, number, number, number, number];

// Mirrors the private `smoothstep`/`zoom_progress` in export/scene/mod.rs. Kept as two functions
// for the same reason Rust splits them: the shrink and the fade MUST ride the identical curve.
const smoothstep = (t: number) => t * t * (3 - 2 * t);
const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

/** TS mirror of `zoom_progress` - 0 at scale 1.0, 1 at `targetScale`. */
export function zoomProgress(scale: number, targetScale: number): number {
  return clamp((scale - 1) / Math.max(targetScale - 1, 0.001), 0, 1);
}

/** TS mirror of `ZoomSettings::resolved_cam_action` (settings/model.rs) - the GLOBAL default,
 *  derived from the legacy `camera_shrink`/`camera_shrink_min` pair when `cam_zoom_default` is
 *  unset, so configs written before that field existed resolve to exactly today's behavior. */
export function resolvedCamDefault(zoom: ZoomSettings): CamZoomAction {
  if (zoom.cam_zoom_default) return zoom.cam_zoom_default;
  return zoom.camera_shrink ? { shrink: { to: zoom.camera_shrink_min } } : "stay";
}

/** TS mirror of `cam_action_at` (export/scene/mod.rs) - the highest-`layer` zoom containing `tMs`
 *  supplies BOTH the action and its OWN `scale` (falling back to `fallback`/`fallbackScale` when
 *  it inherits the action or no zoom is active). Ties go to the LAST such zoom, matching Rust's
 *  `max_by_key`. Returning the zoom's own scale (not just `fallbackScale`) matters: `scale` is a
 *  first-class per-zoom slider (presets 1.6/2.2/2.8), and `zoomProgress`/`camZoomAlpha` divide by
 *  it - feeding a 1.6x zoom the global 2.2x default means it can never reach full shrink/hide. */
export function resolveCamAction(
  zooms: Zoom[], tMs: number, fallback: CamZoomAction, fallbackScale: number,
): [CamZoomAction, number] {
  let best: Zoom | null = null;
  for (const z of zooms) {
    if (tMs < z.start_ms || tMs > z.end_ms) continue;
    if (!best || z.layer >= best.layer) best = z;
  }
  return [best?.cam_action ?? fallback, best?.scale ?? fallbackScale];
}

/** TS mirror of `apply_cam_zoom_action`'s GEOMETRY half (export/scene/mod.rs): `shrink` scales the
 *  panel about its own centre - rect, radius and ring width all by the same smoothstepped factor,
 *  exactly like `shrink_camera`. `hide`/`stay` leave the geometry alone (`hide` is a pure alpha
 *  effect - see `camZoomAlpha`). Scaling about the centre is per-axis here because the tuple is in
 *  fractions, but the multiplier is identical, so it matches the export's pixel-space math. */
export function applyCamZoomAction(
  cam: CamTuple, action: CamZoomAction, scale: number, targetScale: number,
): CamTuple {
  if (action === "hide" || action === "stay") return cam;
  const m = 1 + (clamp(action.shrink.to, 0.1, 1) - 1) * smoothstep(zoomProgress(scale, targetScale));
  const cx = cam[0] + cam[2] / 2, cy = cam[1] + cam[3] / 2;
  const w = cam[2] * m, h = cam[3] * m;
  return [cx - w / 2, cy - h / 2, w, h, cam[4] * m, cam[5] * m, cam[6], cam[7], cam[8]];
}

/** TS mirror of `apply_cam_zoom_action`'s ALPHA half: the factor to multiply the webcam's draw
 *  alpha by. `hide` fades to 0 as the zoom deepens (`1 - smoothstep(z)`, matching the Rust);
 *  every other action leaves it fully opaque. */
export function camZoomAlpha(action: CamZoomAction, scale: number, targetScale: number): number {
  return action === "hide" ? 1 - smoothstep(zoomProgress(scale, targetScale)) : 1;
}
