import type { PreviewLayout, ClickSample } from "../../lib/ipc";
import { drawCursorSprite, type DrawCursor } from "./cursorPreview";

export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }

/** Composite one preview frame: draw the export background + the screen video into its panel
 *  (unzoomed) onto an offscreen buffer, then crop+resize that WHOLE buffer per the camera zoom -
 *  exactly like the export compositor (compositor.rs), which zooms the entire composited scene,
 *  not just the screen's own source, so the background pans/zooms in lockstep with it. The webcam
 *  PiP and cursor are then drawn on top of the zoomed result at their normal (unzoomed) size,
 *  matching the export where they're composited/projected after the crop too. Canvas2D drawImage
 *  holds 60fps. */
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: HTMLImageElement | null, clicks: ClickSample[], now: number,
  cursor: DrawCursor | null, offscreen: HTMLCanvasElement
) {
  if (offscreen.width !== w) offscreen.width = w;
  if (offscreen.height !== h) offscreen.height = h;
  const octx = offscreen.getContext("2d");
  if (!octx) return;

  // Background: the exact export bg image once loaded, else a gradient placeholder.
  if (bg && bg.complete && bg.naturalWidth > 0) {
    octx.drawImage(bg, 0, 0, w, h);
  } else {
    const g = octx.createLinearGradient(0, 0, w * 0.4, h);
    g.addColorStop(0, "#2c2c42"); g.addColorStop(1, "#131318");
    octx.fillStyle = g; octx.fillRect(0, 0, w, h);
  }

  // Screen rect: from the backend layout (exact export framing), else an inset fallback.
  let dx: number, dy: number, dw: number, dh: number, r: number;
  if (layout) {
    dx = layout.screen[0] * w; dy = layout.screen[1] * h;
    dw = layout.screen[2] * w; dh = layout.screen[3] * h; r = layout.radius * w;
  } else {
    const pad = Math.min(w, h) * 0.045;
    dx = pad; dy = pad; dw = w - 2 * pad; dh = h - 2 * pad; r = Math.min(dw, dh) * 0.018 + 6;
  }

  // The screen's full current frame into its panel, unzoomed - the zoom crop below applies to
  // this whole composited base, not to a source-video crop. Gated on the layout's interpolated
  // screenAlpha so a camera_only layout (or a mid-transition frame) hides/fades this panel.
  const vw = screen.videoWidth, vh = screen.videoHeight;
  const screenAlpha = layout?.screenAlpha ?? 1;
  if (vw > 0 && vh > 0 && screenAlpha >= 0.004) {
    octx.globalAlpha = screenAlpha;
    octx.save();
    octx.shadowColor = "rgba(0,0,0,.5)"; octx.shadowBlur = 34; octx.shadowOffsetY = 14;
    roundRect(octx, dx, dy, dw, dh, r); octx.fillStyle = "#000"; octx.fill();
    octx.restore();
    octx.save();
    roundRect(octx, dx, dy, dw, dh, r); octx.clip();
    octx.drawImage(screen, 0, 0, vw, vh, dx, dy, dw, dh);
    octx.restore();
    octx.globalAlpha = 1;
  }

  // Zoom: crop+resize the WHOLE base (background + screen panel together) per the camera, same
  // math as the export's coordmap::crop - so the background pans/zooms with the screen instead of
  // sitting static behind a merely-cropped raw recording.
  const scale = Math.max(cam.scale, 0.01);
  const cw = Math.max(1, w / scale), ch = Math.max(1, h / scale);
  const camPxX = dx + cam.cx * dw, camPxY = dy + cam.cy * dh;
  const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, w - cw));
  const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, h - ch));
  ctx.drawImage(offscreen, cx0, cy0, cw, ch, 0, 0, w, h);

  // Cursor: project its pre-zoom position through the same crop, drawn on the zoomed result at a
  // fixed size (the export doesn't scale cursor size with zoom either - only its position moves).
  if (cursor && vw > 0 && vh > 0) {
    const curPxX = dx + cam.curx * dw, curPxY = dy + cam.cury * dh;
    const cpos: [number, number] = [(curPxX - cx0) * w / cw, (curPxY - cy0) * h / ch];
    drawCursorSprite(ctx, cpos, now, cursor, clicks, h);
  }

  // Webcam PiP: exact rect from the layout (rounded-rect, cover-fit), else a bottom-right circle.
  // Drawn on top of the zoomed result, unzoomed itself - a fixed floating bubble, like the export.
  // Gated on the layout's interpolated camAlpha so a screen_only layout (or a mid-transition
  // frame) hides/fades this PiP.
  const camAlpha = layout?.camAlpha ?? 1;
  if (webcam && webcam.videoWidth > 0 && webcam.videoHeight > 0 && camAlpha >= 0.004) {
    ctx.globalAlpha = camAlpha;
    const wv = webcam.videoWidth, wvh = webcam.videoHeight;
    if (layout?.cam) {
      const [fx, fy, fw, fh, fr] = layout.cam;
      const wx = fx * w, wy = fy * h, ww = fw * w, wh = fh * h, wr = fr * w;
      ctx.save();
      ctx.shadowColor = "rgba(0,0,0,.5)"; ctx.shadowBlur = 20; ctx.shadowOffsetY = 6;
      roundRect(ctx, wx, wy, ww, wh, wr); ctx.fillStyle = "#000"; ctx.fill();
      ctx.restore();
      ctx.save();
      roundRect(ctx, wx, wy, ww, wh, wr); ctx.clip();
      coverDraw(ctx, webcam, wv, wvh, wx, wy, ww, wh);
      ctx.restore();
      roundRect(ctx, wx, wy, ww, wh, wr);
      ctx.strokeStyle = "rgba(255,255,255,.18)"; ctx.lineWidth = 2; ctx.stroke();
    } else {
      const pad = Math.min(w, h) * 0.045;
      const cr = Math.min(w, h) * 0.13;
      const cxp = w - pad - cr - 8, cyp = h - pad - cr - 8;
      const side = Math.min(wv, wvh), wsx = (wv - side) / 2, wsy = (wvh - side) / 2;
      ctx.save();
      ctx.shadowColor = "rgba(0,0,0,.5)"; ctx.shadowBlur = 20; ctx.shadowOffsetY = 6;
      ctx.beginPath(); ctx.arc(cxp, cyp, cr, 0, Math.PI * 2); ctx.fillStyle = "#000"; ctx.fill();
      ctx.restore();
      ctx.save();
      ctx.beginPath(); ctx.arc(cxp, cyp, cr, 0, Math.PI * 2); ctx.clip();
      ctx.drawImage(webcam, wsx, wsy, side, side, cxp - cr, cyp - cr, cr * 2, cr * 2);
      ctx.restore();
      ctx.beginPath(); ctx.arc(cxp, cyp, cr, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(255,255,255,.18)"; ctx.lineWidth = 2; ctx.stroke();
    }
    ctx.globalAlpha = 1;
  }
}

/** Draw `img` to cover the dest rect (centre-crop the source to the dest aspect). */
function coverDraw(ctx: CanvasRenderingContext2D, img: CanvasImageSource, sw: number, sh: number,
  dx: number, dy: number, dw: number, dh: number) {
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale, ch = dh / scale;
  ctx.drawImage(img, (sw - cw) / 2, (sh - ch) / 2, cw, ch, dx, dy, dw, dh);
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  const rr = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.arcTo(x + w, y, x + w, y + h, rr);
  ctx.arcTo(x + w, y + h, x, y + h, rr);
  ctx.arcTo(x, y + h, x, y, rr);
  ctx.arcTo(x, y, x + w, y, rr);
  ctx.closePath();
}
