import type { PreviewLayout } from "../../lib/ipc";
import type { CameraMove, Zoom } from "../../lib/edit";
import type { ZoomSettings } from "../../hud/settings/settings";
import { camMoveAt, overrideCamPanel, type CamPose } from "./cameraMoves";
import { applyCamZoomAction, camZoomAlpha, resolveCamAction, resolvedCamDefault } from "./camZoomAction";

/** The webcam PiP panel for one frame, mirroring `FrameRenderer::step_camera`'s ORDERING - the
 *  single place the preview decides what drives the PiP, so it cannot drift from the export:
 *
 *  1. A `camera_moves` keyframe (or the live Move-mode drag) WINS - but a keyframe only inside
 *     the span it owns (see `camMoveAt`): `overrideCamPanel` replaces the rect and scales radius
 *     + ring width by the height ratio, so a circle stays round. `live` is the layout-resolved
 *     pose this frame, which the track eases out of and back into at the span's edges. Mid-drag
 *     the pointer pose takes precedence over the sampled track (and over the span rule), so the
 *     PiP follows the cursor without writing to the backend every frame (commit is on release).
 *  2. Otherwise - outside the span included - the smart webcam-on-zoom action applies, and only
 *     while the cam panel is narrower than the screen panel, the same guard the export uses.
 *
 *  Returns `base` untouched when there is no camera panel this frame. */
export function frameCamLayout(
  base: PreviewLayout | null, t: number, scale: number,
  moves: CameraMove[], drag: CamPose | null, zooms: Zoom[], zoom: ZoomSettings,
  ow: number, oh: number,
): PreviewLayout | null {
  if (!base?.cam) return base;
  const live: CamPose = {
    x: base.cam[0] + base.cam[2] / 2, y: base.cam[1] + base.cam[3] / 2, size: base.cam[3],
  };
  const cp = drag ?? camMoveAt(moves, t, live);
  if (cp) return { ...base, cam: overrideCamPanel(base.cam, cp, ow, oh) };
  if (base.cam[2] >= base.screen[2]) return base;
  const [action, targetScale] = resolveCamAction(zooms, t, resolvedCamDefault(zoom), zoom.target_scale);
  // `applyCamZoomAction` only ever touches geometry; the alpha half (a pure multiplier, matching
  // Rust's `panel.alpha * (1 - smoothstep(...))`) has to be applied here too, or "Hide" fades the
  // PiP in the export while the preview keeps it fully opaque (see camZoomAlpha's own doc comment).
  return {
    ...base,
    cam: applyCamZoomAction(base.cam, action, scale, targetScale),
    camAlpha: (base.camAlpha ?? 1) * camZoomAlpha(action, scale, targetScale),
  };
}
