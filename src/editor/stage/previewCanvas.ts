import type { PreviewLayout, ClickSample } from "../../lib/ipc";
import { drawCursorSprite, type DrawCursor } from "./cursorPreview";
import { panelFactor, panelClipRect, contentScale } from "./cursorPanel";
import { drawBackground, type StageBgState } from "./stageBg";

export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }

/** Composite one preview frame: draw the export background + the screen video into its panel
 *  (unzoomed) onto an offscreen buffer, then crop+resize that WHOLE buffer per the camera zoom -
 *  exactly like the export compositor (compositor.rs), which zooms the entire composited scene,
 *  not just the screen's own source, so the background pans/zooms in lockstep with it. The webcam
 *  PiP and cursor are then drawn on top of the zoomed result at their normal (unzoomed) size,
 *  matching the export where they're composited/projected after the crop too. Canvas2D drawImage
 *  holds 60fps.
 *
 *  `insetW` is `LayoutPresets.inset_w` (fraction of canvas width) - the export's fixed cursor-scale
 *  reference (see `cursorPanel.ts`); defaults to 1 (no shrink) for the brief window before layout
 *  presets have loaded, same as every other "presets not ready yet" fallback in this file.
 *
 *  `layer` is a scratch canvas `paintPanel` uses to composite a mid-cross-fade panel in one go;
 *  it is only touched while a panel is actually part-transparent. */
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: StageBgState | null, clicks: ClickSample[], now: number,
  cursor: DrawCursor | null, offscreen: HTMLCanvasElement, layer: HTMLCanvasElement, insetW: number = 1,
  bgTimeMs?: number // a video background runs on the output clock; `now` (clip time) is the default
) {
  if (offscreen.width !== w) offscreen.width = w;
  if (offscreen.height !== h) offscreen.height = h;
  const octx = offscreen.getContext("2d");
  if (!octx) return;

  // Background: the exact export bg (a still PNG from the backend, already dimmed there) or, for a
  // video/GIF asset, this frame of it drawn here - see `stageBg.ts` for why those are two paths.
  drawBackground(octx, w, h, bg, bgTimeMs ?? now);

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
    paintPanel(octx, screenAlpha, layer, w, h, (c) => {
      c.save();
      c.shadowColor = "rgba(0,0,0,.5)"; c.shadowBlur = 34; c.shadowOffsetY = 14;
      roundRect(c, dx, dy, dw, dh, r); c.fillStyle = "#000"; c.fill();
      c.restore();
      c.save();
      roundRect(c, dx, dy, dw, dh, r); c.clip();
      c.drawImage(screen, 0, 0, vw, vh, dx, dy, dw, dh);
      c.restore();
    });
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
  // size that scales with the SCREEN PANEL (not the zoom - only its position moves), and clipped
  // to the panel's on-screen rect - mirroring cursorset::draw's `panel` factor + `clip` exactly
  // (see cursorPanel.ts). Both are computed from the panel's own pre-zoom rect (dx/dy/dw/dh, the
  // real layout when loaded or the pad-based fallback otherwise) so they track a shrunk/moved
  // custom-arrangement panel the same way the export does. The CAPTURED cursor also gets
  // `contentScale`: its bitmaps are in SOURCE pixels, so they shrink with the content, not the panel.
  // `screenAlpha >= 0.5` mirrors cursorset::draw's own gate (`if screen.alpha < 0.5 { return; }`):
  // the export stops drawing the synthetic cursor once the screen panel is more than half faded,
  // so a cross-fade into camera_only must not leave a cursor hanging over a panel that has gone.
  if (cursor && vw > 0 && vh > 0 && screenAlpha >= 0.5) {
    const curPxX = dx + cam.curx * dw, curPxY = dy + cam.cury * dh;
    const cpos: [number, number] = [(curPxX - cx0) * w / cw, (curPxY - cy0) * h / ch];
    const panel = panelFactor(dw / w, insetW);
    const clip = panelClipRect({ x: dx, y: dy, w: dw, h: dh }, { cx0, cy0, cw, ch }, w, h);
    drawCursorSprite(ctx, cpos, now, cursor, clicks, h, panel, clip,
      contentScale(panel, insetW * w, cursor.captured?.srcW ?? 0));
  }

  // Webcam PiP: exact rect from the layout (rounded-rect, cover-fit), else a bottom-right circle.
  // Drawn on top of the zoomed result, unzoomed itself - a fixed floating bubble, like the export.
  // Gated on the layout's interpolated camAlpha so a screen_only layout (or a mid-transition
  // frame) hides/fades this PiP.
  // `> 0.004`, not `>=`: the exact threshold `toPreviewLayout` uses to decide whether `layout.cam`
  // is populated at all, so the two can never disagree about a panel at the boundary.
  const camAlpha = layout?.camAlpha ?? 1;
  if (webcam && webcam.videoWidth > 0 && webcam.videoHeight > 0 && camAlpha > 0.004) {
    const wv = webcam.videoWidth, wvh = webcam.videoHeight;
    // Drawn onto `c` - the scratch layer while this panel is mid-fade, the real target when
    // it is opaque - so the bubble, its shadow, its border and its ring composite as ONE.
    paintPanel(ctx, camAlpha, layer, w, h, (c) => {
      if (layout?.cam) {
        const [fx, fy, fw, fh, fr, ringPxFrac, ringR, ringG, ringB] = layout.cam;
        const wx = fx * w, wy = fy * h, ww = fw * w, wh = fh * h, wr = fr * w;
        c.save();
        c.shadowColor = "rgba(0,0,0,.5)"; c.shadowBlur = 20; c.shadowOffsetY = 6;
        roundRect(c, wx, wy, ww, wh, wr); c.fillStyle = "#000"; c.fill();
        c.restore();
        c.save();
        roundRect(c, wx, wy, ww, wh, wr); c.clip();
        coverDraw(c, webcam, wv, wvh, wx, wy, ww, wh);
        c.restore();
        roundRect(c, wx, wy, ww, wh, wr);
        c.strokeStyle = "rgba(255,255,255,.18)"; c.lineWidth = 2; c.stroke();
        // Export ring/border (Panel.ring_px/ring_color): a band `ringPx` wide, just INSIDE the
        // panel edge - matches shader.wgsl/compositor.rs, whose SDF band spans d in [-ringPx, 0].
        // A centered stroke traced on a path inset by ringPx/2 straddles exactly that band (outer
        // half of the stroke lands on the true edge, inner half ringPx further in).
        const ringPx = ringPxFrac * w;
        if (ringPx > 0) {
          const inset = ringPx / 2;
          roundRect(c, wx + inset, wy + inset, ww - 2 * inset, wh - 2 * inset, Math.max(0, wr - inset));
          c.strokeStyle = `rgb(${ringR}, ${ringG}, ${ringB})`; c.lineWidth = ringPx; c.stroke();
        }
      } else if (!layout) {
        // Only when there is NO layout at all. A layout that exists and simply hides the webcam
        // (screen_only, or the static `preview_layout` before presets land - it carries no alpha
        // field, so `camAlpha` reads 1) must draw NOTHING, not this hardcoded corner bubble.
        const pad = Math.min(w, h) * 0.045;
        const cr = Math.min(w, h) * 0.13;
        const cxp = w - pad - cr - 8, cyp = h - pad - cr - 8;
        const side = Math.min(wv, wvh), wsx = (wv - side) / 2, wsy = (wvh - side) / 2;
        c.save();
        c.shadowColor = "rgba(0,0,0,.5)"; c.shadowBlur = 20; c.shadowOffsetY = 6;
        c.beginPath(); c.arc(cxp, cyp, cr, 0, Math.PI * 2); c.fillStyle = "#000"; c.fill();
        c.restore();
        c.save();
        c.beginPath(); c.arc(cxp, cyp, cr, 0, Math.PI * 2); c.clip();
        c.drawImage(webcam, wsx, wsy, side, side, cxp - cr, cyp - cr, cr * 2, cr * 2);
        c.restore();
        c.beginPath(); c.arc(cxp, cyp, cr, 0, Math.PI * 2);
        c.strokeStyle = "rgba(255,255,255,.18)"; c.lineWidth = 2; c.stroke();
      }
    });
  }
}

/** Composite one panel ONCE at `alpha`, the way the export's shader does - a single
 *  `mix(color, panel, cov * panel_a)` per panel (gpu/shader.wgsl). Setting `globalAlpha` and then
 *  drawing the panel's own layers straight onto `target` composites EACH of them separately: the
 *  shadow backing and the video both land at `alpha`, so a panel at alpha 0.5 came out as
 *  `0.5*video + 0.25*bg + 0.25*black` - visibly DARK through every layout cross-fade, and worst at
 *  the middle of it, which the export never does. Painting them into a scratch layer and blitting
 *  that once is the same picture the shader produces (plus the preview's decorative shadow, which
 *  now fades WITH its panel instead of as a layer of its own).
 *
 *  Opaque is the overwhelmingly common case and takes the direct path, so the extra full-canvas
 *  buffer is only ever touched on the frames a transition is actually running. */
function paintPanel(target: CanvasRenderingContext2D, alpha: number, scratch: HTMLCanvasElement,
  w: number, h: number, draw: (c: CanvasRenderingContext2D) => void) {
  if (alpha >= 0.999) { draw(target); return; }
  if (scratch.width !== w) scratch.width = w;
  if (scratch.height !== h) scratch.height = h;
  const sc = scratch.getContext("2d");
  if (!sc) { draw(target); return; } // no context: an undimmed panel beats no panel
  sc.clearRect(0, 0, w, h);
  draw(sc);
  target.save();
  target.globalAlpha = alpha;
  target.drawImage(scratch, 0, 0);
  target.restore();
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
