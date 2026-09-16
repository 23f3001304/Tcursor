import type { ClickSample } from "../../../shared/ipc";
import { drawCursorSprite, type DrawCursor } from "../cursor/cursorPreview";
import { panelFactor, panelClipRect, contentScale } from "../cursor/cursorPanel";
import type { DrawCam, PreviewGeom } from "./previewCanvas";

export function drawCursorLayer(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  cam: DrawCam,
  cursor: DrawCursor | null,
  clicks: ClickSample[],
  now: number,
  insetW: number,
  g: PreviewGeom,
) {
  if (!cursor || !g.hasVideo || g.screenAlpha < 0.5) return;
  const [dx, dy, dw, dh] = g.panel;
  const [cx0, cy0, cw, ch] = g.crop;
  const cpos: [number, number] = [
    ((dx + cam.curx * dw - cx0) * w) / cw,
    ((dy + cam.cury * dh - cy0) * h) / ch,
  ];
  const panel = panelFactor(dw / w, insetW);
  const clip = panelClipRect({ x: dx, y: dy, w: dw, h: dh }, { cx0, cy0, cw, ch }, w, h);
  drawCursorSprite(
    ctx,
    cpos,
    now,
    cursor,
    clicks,
    h,
    panel,
    clip,
    contentScale(panel, insetW * w, (cursor.captured?.srcW ?? 0) * g.src[2]),
  );
}
