import type { PreviewLayout, ClickSample, CursorKindSample } from "../lib/ipc";
import { spotlightAlpha, drawSpotlight, type SpotlightInput } from "./spotlightPreview";

export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }

/** Everything the preview needs to draw the export cursor: the recording's cursor style/size +
 *  bounce, the type track, and the decoded sprite images (+ hotspots + canvas heights) by kind. */
export interface DrawCursor {
  style: string; size: number; clickBounce: boolean; bounceIntensity: number; motionBlur: number;
  kinds: CursorKindSample[];
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  recent: [number, number][]; // ring of recent on-canvas positions, for the motion trail
}

const RIPPLE_MS = 500;

/** Composite one preview frame onto a 2D canvas: the export background, the zoomed screen video
 *  with cursor + click ripples + spotlight, and the webcam PiP. Framing + background come from the
 *  backend so the preview approximates the export (the zoom magnifies the screen within its fixed
 *  panel, so framing differs at high zoom). Canvas2D drawImage holds 60fps. */
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: HTMLImageElement | null, clicks: ClickSample[], now: number,
  cursor: DrawCursor | null, spotlight: SpotlightInput | null,
) {
  // Background: the exact export bg image once loaded, else a gradient placeholder.
  if (bg && bg.complete && bg.naturalWidth > 0) {
    ctx.drawImage(bg, 0, 0, w, h);
  } else {
    const g = ctx.createLinearGradient(0, 0, w * 0.4, h);
    g.addColorStop(0, "#2c2c42"); g.addColorStop(1, "#131318");
    ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
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

  let curC: [number, number] | null = null, cpos: [number, number] | null = null; // spotlight centre; cursor pos
  const vw = screen.videoWidth, vh = screen.videoHeight;
  if (vw > 0 && vh > 0) {
    // Zoom is a source-rect crop centred on cam.cx/cy.
    const s = Math.max(1, cam.scale);
    const sw = vw / s, sh = vh / s;
    const sx = Math.max(0, Math.min(vw - sw, cam.cx * vw - sw / 2));
    const sy = Math.max(0, Math.min(vh - sh, cam.cy * vh - sh / 2));
    ctx.save();
    ctx.shadowColor = "rgba(0,0,0,.5)"; ctx.shadowBlur = 34; ctx.shadowOffsetY = 14;
    roundRect(ctx, dx, dy, dw, dh, r); ctx.fillStyle = "#000"; ctx.fill();
    ctx.restore();
    ctx.save();
    roundRect(ctx, dx, dy, dw, dh, r); ctx.clip();
    ctx.drawImage(screen, sx, sy, sw, sh, dx, dy, dw, dh);
    // Map a 0..1 screen-content point to its on-canvas pixel through the zoom crop, or null
    // when it lies outside the visible crop. Cursor + click ripples share this so they track.
    const map = (fx: number, fy: number): [number, number] | null => {
      const vx = fx * vw, vy = fy * vh;
      if (vx < sx || vx > sx + sw || vy < sy || vy > sy + sh) return null;
      return [dx + ((vx - sx) / sw) * dw, dy + ((vy - sy) / sh) * dh];
    };
    drawClicks(ctx, clicks, now, map, Math.min(w, h)); // ripples behind the cursor, clipped to screen
    cpos = map(cam.curx, cam.cury);
    curC = [dx + ((cam.curx * vw - sx) / sw) * dw, dy + ((cam.cury * vh - sy) / sh) * dh];
    ctx.restore();
    // Cursor on top, UNCLIPPED, so it is never cut at the panel edge/rounded corner when zoomed
    // (the export's panel-rect cursor clip expands to ~the whole frame at zoom, so it isn't cut).
    if (cpos && cursor) drawCursorSprite(ctx, cpos, now, cursor, clicks, h);
  }

  // Webcam PiP: exact rect from the layout (rounded-rect, cover-fit), else a bottom-right circle.
  if (webcam && webcam.videoWidth > 0 && webcam.videoHeight > 0) {
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
  }

  // Spotlight: darken the whole canvas with a soft hole at the cursor (approximates the export's Classic mode).
  if (spotlight && curC) drawSpotlight(ctx, w, h, curC, spotlightAlpha(spotlight.effects, spotlight.holds, spotlight.on, now), spotlight.params);
}

/** Draw `img` to cover the dest rect (centre-crop the source to the dest aspect). */
function coverDraw(ctx: CanvasRenderingContext2D, img: CanvasImageSource, sw: number, sh: number,
  dx: number, dy: number, dw: number, dh: number) {
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale, ch = dh / scale;
  ctx.drawImage(img, (sw - cw) / 2, (sh - ch) / 2, cw, ch, dx, dy, dw, dh);
}

/** Expanding click ripples: for each click within RIPPLE_MS of `now`, a fading ring at the
 *  zoom-mapped click position (matching the export's click FX). `map` projects 0..1 screen
 *  points through the current zoom crop (null = off-screen). */
function drawClicks(ctx: CanvasRenderingContext2D, clicks: ClickSample[], now: number,
  map: (fx: number, fy: number) => [number, number] | null, minSide: number) {
  for (const c of clicks) {
    const dt = now - c.t;
    if (dt < 0 || dt > RIPPLE_MS) continue;
    const p = map(c.x, c.y);
    if (!p) continue;
    const t = dt / RIPPLE_MS;
    const radius = minSide * (0.012 + 0.05 * t);
    const a = (1 - t) * 0.5;
    ctx.beginPath(); ctx.arc(p[0], p[1], radius, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(255,255,255,${a * 0.35})`; ctx.fill();
    ctx.lineWidth = 2; ctx.strokeStyle = `rgba(255,255,255,${a})`; ctx.stroke();
  }
}

/** The cursor type active at output time `ms` (last sample with t <= ms), default "arrow". */
function cursorAt(kinds: CursorKindSample[], ms: number): string {
  let lo = 0, hi = kinds.length;
  while (lo < hi) { const mid = (lo + hi) >> 1; if (kinds[mid].t <= ms) lo = mid + 1; else hi = mid; }
  return lo > 0 ? kinds[lo - 1].kind : "arrow";
}

/** Draw the real export cursor sprite at the mapped position `p`, gated by style: only when
 *  "enhanced" (System = the OS cursor is already in the video; Hidden = none). Sized like the
 *  export (size * outH * 0.033, uniform on the sprite's canvas height), anchored at the hotspot,
 *  with the same post-click bounce dip and a fading motion trail (driven by `motionBlur`). */
function drawCursorSprite(ctx: CanvasRenderingContext2D, p: [number, number], now: number,
  c: DrawCursor, clicks: ClickSample[], outH: number) {
  if (c.style !== "enhanced") return;
  const kind = cursorAt(c.kinds, now);
  const img = c.sprites.get(kind) ?? c.sprites.get("arrow");
  const ch = c.canvasH.get(kind) ?? c.canvasH.get("arrow");
  const hot = c.hots.get(kind) ?? c.hots.get("arrow");
  if (!img || !img.complete || !img.naturalWidth || !ch || !hot) return;
  let sizePx = c.size * outH * 0.033;
  if (c.clickBounce) {
    for (let i = clicks.length - 1; i >= 0; i--) {
      const dt = now - clicks[i].t;
      if (dt < 0) continue;
      if (dt < 180) sizePx *= 1 - 0.36 * c.bounceIntensity * (1 - dt / 180);
      break;
    }
  }
  const scale = sizePx / ch;
  const tw = img.naturalWidth * scale, th = img.naturalHeight * scale;
  const blit = (q: [number, number]) => ctx.drawImage(img, q[0] - hot[0] * tw, q[1] - hot[1] * th, tw, th);
  // Motion trail (matches the export's apply_enhanced): the last few positions, fading. A large
  // jump (a scrub) clears it so the trail never smears across a seek.
  const last = c.recent[c.recent.length - 1];
  if (last && Math.hypot(p[0] - last[0], p[1] - last[1]) > outH * 0.2) c.recent.length = 0;
  c.recent.push(p); if (c.recent.length > 6) c.recent.shift();
  if (c.motionBlur > 0) {
    const trail = c.recent.slice(0, -1).reverse();
    for (let i = 0; i < trail.length; i++) {
      ctx.globalAlpha = Math.min(1, c.motionBlur * (1 - i / trail.length) * 0.5);
      blit(trail[i]);
    }
    ctx.globalAlpha = 1;
  }
  blit(p);
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
