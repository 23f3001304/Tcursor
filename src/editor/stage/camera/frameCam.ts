import type { PreviewLayout } from "../../../shared/ipc";
import type { CameraMove, Zoom } from "../../../shared/edit";
import type { ZoomSettings } from "../../../hud/settings/settings";
import { camMoveAt, liveCamPose, overrideCamPanel, type CamPose } from "./cameraMoves";
import { applyCamZoomAction, camZoomAlpha, resolveCamAction, resolvedCamDefault } from "./camZoomAction";

export const activeCamDraft = (draft: CamPose | null, arranging: boolean): CamPose | null =>
  arranging ? null : draft;

export function frameCamLayout(
  base: PreviewLayout | null,
  t: number,
  scale: number,
  moves: CameraMove[],
  drag: CamPose | null,
  zooms: Zoom[],
  zoom: ZoomSettings,
  ow: number,
  oh: number,
): PreviewLayout | null {
  if (!base?.cam) return base;
  const live = liveCamPose(base.cam, ow, oh);
  const cp = drag ?? camMoveAt(moves, t, live);
  if (cp) return { ...base, cam: overrideCamPanel(base.cam, cp, ow, oh) };
  if (base.cam[2] >= base.screen[2]) return base;
  const [action, targetScale] = resolveCamAction(zooms, t, resolvedCamDefault(zoom), zoom.target_scale);
  return {
    ...base,
    cam: applyCamZoomAction(base.cam, action, scale, targetScale),
    camAlpha: (base.camAlpha ?? 1) * camZoomAlpha(action, scale, targetScale),
  };
}
