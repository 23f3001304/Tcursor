import type { PreviewLayout } from "../../../shared/ipc";
import { FULL_SRC } from "./sourceSpans";
import { drawBackground, type StageBgState } from "./stageBg";
import { coverDraw, paintPanel, roundRect } from "./previewDraw";

export { coverDraw, paintPanel, roundRect };

export interface DrawCam {
  scale: number;
  cx: number;
  cy: number;
  curx: number;
  cury: number;
}

export interface PreviewGeom {
  panel: [number, number, number, number];
  crop: [number, number, number, number];
  src: [number, number, number, number];
  screenAlpha: number;
  hasVideo: boolean;
}

export function drawPreview(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  screen: HTMLVideoElement,
  webcam: HTMLVideoElement | null,
  cam: DrawCam,
  layout: PreviewLayout | null,
  bg: StageBgState | null,
  now: number,
  offscreen: HTMLCanvasElement,
  layer: HTMLCanvasElement,
  bgTimeMs?: number,
): PreviewGeom | null {
  if (offscreen.width !== w) offscreen.width = w;
  if (offscreen.height !== h) offscreen.height = h;
  const octx = offscreen.getContext("2d");
  if (!octx) return null;

  drawBackground(octx, w, h, bg, bgTimeMs ?? now);

  let dx: number, dy: number, dw: number, dh: number, r: number;
  if (layout) {
    dx = layout.screen[0] * w;
    dy = layout.screen[1] * h;
    dw = layout.screen[2] * w;
    dh = layout.screen[3] * h;
    r = layout.radius * w;
  } else {
    const pad = Math.min(w, h) * 0.045;
    dx = pad;
    dy = pad;
    dw = w - 2 * pad;
    dh = h - 2 * pad;
    r = Math.min(dw, dh) * 0.018 + 6;
  }

  const vw = screen.videoWidth,
    vh = screen.videoHeight;
  const screenAlpha = layout?.screenAlpha ?? 1;
  const src = layout?.src ?? FULL_SRC;
  if (vw > 0 && vh > 0 && screenAlpha >= 0.004) {
    paintPanel(octx, screenAlpha, layer, w, h, (c) => {
      c.save();
      c.shadowColor = "rgba(0,0,0,.5)";
      c.shadowBlur = 34;
      c.shadowOffsetY = 14;
      roundRect(c, dx, dy, dw, dh, r);
      c.fillStyle = "#000";
      c.fill();
      c.restore();
      c.save();
      roundRect(c, dx, dy, dw, dh, r);
      c.clip();
      c.drawImage(screen, src[0] * vw, src[1] * vh, src[2] * vw, src[3] * vh, dx, dy, dw, dh);
      c.restore();
    });
  }

  const scale = Math.max(cam.scale, 0.01);
  const cw = Math.max(1, w / scale),
    ch = Math.max(1, h / scale);
  const camPxX = dx + cam.cx * dw,
    camPxY = dy + cam.cy * dh;
  const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, w - cw));
  const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, h - ch));
  ctx.drawImage(offscreen, cx0, cy0, cw, ch, 0, 0, w, h);

  const camAlpha = layout?.camAlpha ?? 1;
  if (webcam && webcam.videoWidth > 0 && webcam.videoHeight > 0 && camAlpha > 0.004) {
    const wv = webcam.videoWidth,
      wvh = webcam.videoHeight;
    paintPanel(ctx, camAlpha, layer, w, h, (c) => {
      if (layout?.cam) {
        const [fx, fy, fw, fh, fr, ringPxFrac, ringR, ringG, ringB] = layout.cam;
        const wx = fx * w,
          wy = fy * h,
          ww = fw * w,
          wh = fh * h,
          wr = fr * w;
        c.save();
        c.shadowColor = "rgba(0,0,0,.5)";
        c.shadowBlur = 20;
        c.shadowOffsetY = 6;
        roundRect(c, wx, wy, ww, wh, wr);
        c.fillStyle = "#000";
        c.fill();
        c.restore();
        c.save();
        roundRect(c, wx, wy, ww, wh, wr);
        c.clip();
        coverDraw(c, webcam, wv, wvh, wx, wy, ww, wh);
        c.restore();
        roundRect(c, wx, wy, ww, wh, wr);
        c.strokeStyle = "rgba(255,255,255,.18)";
        c.lineWidth = 2;
        c.stroke();
        const ringPx = ringPxFrac * w;
        if (ringPx > 0) {
          const inset = ringPx / 2;
          roundRect(c, wx + inset, wy + inset, ww - 2 * inset, wh - 2 * inset, Math.max(0, wr - inset));
          c.strokeStyle = `rgb(${ringR}, ${ringG}, ${ringB})`;
          c.lineWidth = ringPx;
          c.stroke();
        }
      } else if (!layout) {
        const pad = Math.min(w, h) * 0.045;
        const cr = Math.min(w, h) * 0.13;
        const cxp = w - pad - cr - 8,
          cyp = h - pad - cr - 8;
        const side = Math.min(wv, wvh),
          wsx = (wv - side) / 2,
          wsy = (wvh - side) / 2;
        c.save();
        c.shadowColor = "rgba(0,0,0,.5)";
        c.shadowBlur = 20;
        c.shadowOffsetY = 6;
        c.beginPath();
        c.arc(cxp, cyp, cr, 0, Math.PI * 2);
        c.fillStyle = "#000";
        c.fill();
        c.restore();
        c.save();
        c.beginPath();
        c.arc(cxp, cyp, cr, 0, Math.PI * 2);
        c.clip();
        c.drawImage(webcam, wsx, wsy, side, side, cxp - cr, cyp - cr, cr * 2, cr * 2);
        c.restore();
        c.beginPath();
        c.arc(cxp, cyp, cr, 0, Math.PI * 2);
        c.strokeStyle = "rgba(255,255,255,.18)";
        c.lineWidth = 2;
        c.stroke();
      }
    });
  }
  return {
    panel: [dx, dy, dw, dh],
    crop: [cx0, cy0, cw, ch],
    src,
    screenAlpha,
    hasVideo: vw > 0 && vh > 0,
  };
}
