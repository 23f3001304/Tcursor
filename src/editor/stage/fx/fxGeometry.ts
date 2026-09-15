import type { PreviewLayout } from "../../../shared/ipc";
import { FULL_SRC, toPanelFrac } from "../canvas/sourceSpans";
import type { FxCamRect } from "./fxOverlay";

export interface FxFrameGeometry {
  fxW: number;
  fxH: number;
  screenScale: number;
  map: (fx: number, fy: number) => [number, number] | null;
  mapCanvas: (cx: number, cy: number) => [number, number] | null;
}

export function fxFrameGeometry(
  canvasW: number,
  canvasH: number,
  lay: PreviewLayout | null,
  cam: { cx: number; cy: number; scale: number },
  fxScale: number,
): FxFrameGeometry {
  const fxW = Math.max(1, Math.round(canvasW * fxScale)),
    fxH = Math.max(1, Math.round(canvasH * fxScale));
  const pad = Math.min(fxW, fxH) * 0.045;
  const dx = lay ? lay.screen[0] * fxW : pad,
    dy = lay ? lay.screen[1] * fxH : pad;
  const dw = lay ? lay.screen[2] * fxW : fxW - 2 * pad,
    dh = lay ? lay.screen[3] * fxH : fxH - 2 * pad;
  const scale = Math.max(cam.scale, 0.01);
  const cw = Math.max(1, Math.round(fxW / scale)),
    ch = Math.max(1, Math.round(fxH / scale));
  const camPxX = dx + cam.cx * dw,
    camPxY = dy + cam.cy * dh;
  const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, fxW - cw));
  const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, fxH - ch));
  const src = lay?.src ?? FULL_SRC;
  const map = (fx: number, fy: number): [number, number] => {
    const bx = dx + fx * dw,
      by = dy + fy * dh;
    return [((bx - cx0) * fxW) / cw, ((by - cy0) * fxH) / ch];
  };
  return {
    fxW,
    fxH,
    screenScale: fxH > 0 ? dh / fxH : 1,
    map,
    mapCanvas: (cx: number, cy: number) => {
      const [px, py] = toPanelFrac(cx, cy, src);
      return map(px, py);
    },
  };
}

export function fxCamRect(lay: PreviewLayout | null, fxW: number, fxH: number): FxCamRect {
  if (!lay?.cam || (lay.camAlpha ?? 1) <= 0.05) return null;
  return {
    rect: [
      lay.cam[0] * fxW,
      lay.cam[1] * fxH,
      (lay.cam[0] + lay.cam[2]) * fxW,
      (lay.cam[1] + lay.cam[3]) * fxH,
    ],
    radius: lay.cam[4] * fxW,
  };
}
