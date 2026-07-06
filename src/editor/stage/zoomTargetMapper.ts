import type { PreviewLayout } from "../../lib/ipc";

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
  canvasElement: HTMLCanvasElement;
  layout: PreviewLayout | null;
  cam: { scale: number; cx: number; cy: number };
}): [number, number] | null {
  const c = canvasElement;
  const w = c.width, h = c.height;
  const rect = c.getBoundingClientRect();
  const canvasPxX = ((clientX - rect.left) / rect.width) * w;
  const canvasPxY = ((clientY - rect.top) / rect.height) * h;

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

  // Un-project the click through that crop back to the pre-zoom (panel-local) point.
  const bx = cx0 + (canvasPxX / w) * cw;
  const by = cy0 + (canvasPxY / h) * ch;

  if (bx < dx || bx > dx + dw || by < dy || by > dy + dh) return null;

  return [(bx - dx) / dw, (by - dy) / dh];
}
