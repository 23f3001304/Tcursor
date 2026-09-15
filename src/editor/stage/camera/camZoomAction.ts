import type { CamZoomAction, Zoom } from "../../../shared/edit";
import type { ZoomSettings } from "../../../hud/settings/settings";

export type CamTuple = [number, number, number, number, number, number, number, number, number];

const smoothstep = (t: number) => t * t * (3 - 2 * t);
const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

export function zoomProgress(scale: number, targetScale: number): number {
  return clamp((scale - 1) / Math.max(targetScale - 1, 0.001), 0, 1);
}

export function resolvedCamDefault(zoom: ZoomSettings): CamZoomAction {
  if (zoom.cam_zoom_default) return zoom.cam_zoom_default;
  return zoom.camera_shrink ? { shrink: { to: zoom.camera_shrink_min } } : "stay";
}

export function resolveCamAction(
  zooms: Zoom[],
  tMs: number,
  fallback: CamZoomAction,
  fallbackScale: number,
): [CamZoomAction, number] {
  let best: Zoom | null = null;
  for (const z of zooms) {
    if (tMs < z.start_ms || tMs > z.end_ms) continue;
    if (!best || z.layer >= best.layer) best = z;
  }
  return [best?.cam_action ?? fallback, best?.scale ?? fallbackScale];
}

export function applyCamZoomAction(
  cam: CamTuple,
  action: CamZoomAction,
  scale: number,
  targetScale: number,
): CamTuple {
  if (action === "hide" || action === "stay") return cam;
  const m = 1 + (clamp(action.shrink.to, 0.1, 1) - 1) * smoothstep(zoomProgress(scale, targetScale));
  const cx = cam[0] + cam[2] / 2,
    cy = cam[1] + cam[3] / 2;
  const w = cam[2] * m,
    h = cam[3] * m;
  return [cx - w / 2, cy - h / 2, w, h, cam[4] * m, cam[5] * m, cam[6], cam[7], cam[8]];
}

export function camZoomAlpha(action: CamZoomAction, scale: number, targetScale: number): number {
  return action === "hide" ? 1 - smoothstep(zoomProgress(scale, targetScale)) : 1;
}
