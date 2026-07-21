import type { PreviewLayout } from "../../lib/ipc";
import type { CameraMove, Zoom } from "../../lib/edit";
import type { ZoomSettings } from "../../hud/settings/settings";
import { camMoveAt, overrideCamPanel, type CamPose } from "./cameraMoves";
import { applyCamZoomAction, resolveCamAction, resolvedCamDefault } from "./camZoomAction";

/** The webcam PiP panel for one frame, mirroring `FrameRenderer::step_camera`'s ORDERING - the
 *  single place the preview decides what drives the PiP, so it cannot drift from the export:
 *
 *  1. A `camera_moves` keyframe (or the live Move-mode drag) WINS: `overrideCamPanel` replaces
 *     the rect and scales radius + ring width by the height ratio, so a circle stays round.
 *     `staticPose` is the implicit t=0 keyframe a lone keyframe eases in from. Mid-drag the
 *     pointer pose takes precedence over the sampled track, so the PiP follows the cursor
 *     without writing to the backend every frame (the drag only commits on release).
 *  2. Otherwise the smart webcam-on-zoom action applies - and only while the cam panel is
 *     narrower than the screen panel, the same guard the export uses.
 *
 *  Returns `base` untouched when there is no camera panel this frame. */
export function frameCamLayout(
  base: PreviewLayout | null, t: number, scale: number,
  moves: CameraMove[], drag: CamPose | null, zooms: Zoom[], zoom: ZoomSettings,
  ow: number, oh: number,
): PreviewLayout | null {
  if (!base?.cam) return base;
  const staticPose: CamPose = {
    x: base.cam[0] + base.cam[2] / 2, y: base.cam[1] + base.cam[3] / 2, size: base.cam[3],
  };
  const cp = drag ?? camMoveAt(moves, t, staticPose);
  if (cp) return { ...base, cam: overrideCamPanel(base.cam, cp, ow, oh) };
  if (base.cam[2] >= base.screen[2]) return base;
  const action = resolveCamAction(zooms, t, resolvedCamDefault(zoom));
  return { ...base, cam: applyCamZoomAction(base.cam, action, scale, zoom.target_scale) };
}
