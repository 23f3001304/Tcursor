export function panelFactor(screenW: number, insetW: number): number {
  const f = screenW / Math.max(insetW, 1e-6);
  return Math.min(1, Math.max(0.1, f));
}

export function contentScale(panel: number, insetPx: number, srcW: number): number {
  return srcW > 0 ? (panel * insetPx) / srcW : panel;
}

export interface ZoomCrop {
  cx0: number;
  cy0: number;
  cw: number;
  ch: number;
}

export function panelClipRect(
  rect: { x: number; y: number; w: number; h: number },
  crop: ZoomCrop,
  canvasW: number,
  canvasH: number,
): [number, number, number, number] {
  const project = (px: number, py: number): [number, number] => [
    ((px - crop.cx0) * canvasW) / crop.cw,
    ((py - crop.cy0) * canvasH) / crop.ch,
  ];
  const [x0, y0] = project(rect.x, rect.y);
  const [x1, y1] = project(rect.x + rect.w, rect.y + rect.h);
  return [Math.max(0, x0), Math.max(0, y0), Math.min(canvasW, x1), Math.min(canvasH, y1)];
}
