import type { CameraMove, LayoutSeg, Zoom } from "../../../shared/edit";
import type { CamSample, LayoutPresets, PreviewLayout } from "../../../shared/ipc";
import type { ZoomSettings } from "../../../hud/settings/settings";
import { layoutAt } from "../../timeline/model/layoutTrack";
import { camAt, type Cam } from "../camera/camera";
import { frameCamLayout } from "../camera/frameCam";
import type { CamPose } from "../camera/cameraMoves";

export interface MaskFrameScene {
  track: CamSample[];
  layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null;
  layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[];
  zooms: Zoom[];
  zoomSettings: ZoomSettings;
}

export interface MaskFrame {
  layout: PreviewLayout | null;
  cam: Cam;
}

export function maskFrameAt(
  s: MaskFrameScene,
  tOut: number,
  canvasW: number,
  canvasH: number,
  drag: CamPose | null,
): MaskFrame {
  const cam = camAt(s.track, tOut);
  const base = layoutAt(s.layoutSegs, s.layoutPresets, tOut, [canvasW, canvasH]) ?? s.layout;
  return {
    layout: frameCamLayout(
      base,
      tOut,
      cam.scale,
      s.cameraMoves,
      drag,
      s.zooms,
      s.zoomSettings,
      canvasW,
      canvasH,
    ),
    cam,
  };
}
