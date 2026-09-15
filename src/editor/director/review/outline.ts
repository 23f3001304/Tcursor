import type { PreviewLayout } from "../../../shared/ipc";
import { mapZoomTargetToCanvasPoint, type StageCam } from "../../stage/camera/zoomTargetMapper";

export interface OutlineBox {
  left: number;
  top: number;
  width: number;
  height: number;
}

const round4 = (v: number) => Math.round(v * 1e4) / 1e4;

export function outlineRect(
  rect: [number, number, number, number],
  canvasW: number,
  canvasH: number,
  layout: PreviewLayout | null,
  cam: StageCam,
): OutlineBox | null {
  const [x, y, w, h] = rect;
  if (![x, y, w, h].every(Number.isFinite) || w <= 0 || h <= 0) return null;
  const at = (tx: number, ty: number) =>
    mapZoomTargetToCanvasPoint({ tx, ty, canvasW, canvasH, layout, cam });
  const [x0, y0] = at(x, y);
  const [x1, y1] = at(x + w, y + h);
  const left = Math.min(x0, x1),
    top = Math.min(y0, y1);
  const width = Math.abs(x1 - x0),
    height = Math.abs(y1 - y0);
  if (left + width <= 0 || left >= 1 || top + height <= 0 || top >= 1) return null;
  return { left: round4(left), top: round4(top), width: round4(width), height: round4(height) };
}
