import type { ZoomTarget } from "../../lib/edit";
import type { PreviewLayout } from "../../lib/ipc";

export interface StageCam { scale: number; cx: number; cy: number }

/** The screen panel rect + the whole-frame zoom crop, both in canvas pixels, for a given
 *  frame. The single source both directions of the click<->target mapping below read. */
function stageCrop(w: number, h: number, layout: PreviewLayout | null, cam: StageCam) {
  const pad = Math.min(w, h) * 0.045;
  const dx = layout ? layout.screen[0] * w : pad;
  const dy = layout ? layout.screen[1] * h : pad;
  const dw = layout ? layout.screen[2] * w : w - 2 * pad;
  const dh = layout ? layout.screen[3] * h : h - 2 * pad;
  // The same zoom crop drawPreview applies to the whole composited base frame.
  const scale = Math.max(cam.scale, 0.01);
  const cw = Math.max(1, w / scale), ch = Math.max(1, h / scale);
  const camPxX = dx + cam.cx * dw, camPxY = dy + cam.cy * dh;
  const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, w - cw));
  const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, h - ch));
  return { dx, dy, dw, dh, cw, ch, cx0, cy0 };
}

/** Invert a canvas click into a 0..1 screen-content fraction (the zoom target `update_zoom`
 *  expects), un-projecting through the CURRENT zoom exactly like the compositor's whole-frame
 *  crop (coordmap::crop) - not a crop of the raw screen source - so clicking to add a zoom while
 *  already zoomed in targets the same point the user sees, instead of a point in a static panel
 *  crop that no longer matches what's rendered. */
export function mapCanvasClickToZoomTarget({
  clientX,
  clientY,
  canvasElement,
  layout,
  cam,
}: {
  clientX: number;
  clientY: number;
  canvasElement: { width: number; height: number; getBoundingClientRect(): { left: number; top: number; width: number; height: number } };
  layout: PreviewLayout | null;
  cam: StageCam;
}): [number, number] | null {
  const c = canvasElement;
  const w = c.width, h = c.height;
  const rect = c.getBoundingClientRect();
  const canvasPxX = ((clientX - rect.left) / rect.width) * w;
  const canvasPxY = ((clientY - rect.top) / rect.height) * h;

  const { dx, dy, dw, dh, cw, ch, cx0, cy0 } = stageCrop(w, h, layout, cam);

  // Un-project the click through that crop back to the pre-zoom (panel-local) point.
  const bx = cx0 + (canvasPxX / w) * cw;
  const by = cy0 + (canvasPxY / h) * ch;

  if (bx < dx || bx > dx + dw || by < dy || by > dy + dh) return null;

  return [(bx - dx) / dw, (by - dy) / dh];
}

/** The exact forward of `mapCanvasClickToZoomTarget`: where a stored zoom target lands on the
 *  canvas RIGHT NOW, as 0..1 fractions of the canvas' displayed box (which `.e-stage` matches
 *  exactly, so they convert straight to CSS percentages). Values outside 0..1 mean the aim point
 *  is currently cropped out of frame - the caller decides whether to still draw it. */
export function mapZoomTargetToCanvasPoint({ tx, ty, canvasW, canvasH, layout, cam }: {
  tx: number; ty: number; canvasW: number; canvasH: number;
  layout: PreviewLayout | null; cam: StageCam;
}): [number, number] {
  const { dx, dy, dw, dh, cw, ch, cx0, cy0 } = stageCrop(canvasW, canvasH, layout, cam);
  const bx = dx + tx * dw, by = dy + ty * dh;
  return [(bx - cx0) / cw, (by - cy0) / ch];
}

/** The aim point of a `ZoomTarget`, or `null` when it follows the live cursor (no stored point). */
export function zoomTargetPoint(target: ZoomTarget | undefined | null): [number, number] | null {
  return target && typeof target === "object" && "fixed" in target ? [target.fixed.x, target.fixed.y] : null;
}
