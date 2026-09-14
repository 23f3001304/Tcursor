import type { PreviewLayout } from "../../lib/ipc";
import { FULL_SRC, toPanelFrac } from "./sourceSpans";
import type { FxCamRect } from "./fxOverlay";

/** The pure geometry the FX overlay needs, pulled out of `useCompositeLoop`'s rAF tick so it is
 *  unit-testable without a canvas (and so that hook stays inside the file-size budget). Everything
 *  here works in the DOWNSCALED FX-render space (`fxW` x `fxH` = the canvas times `fxScale`), which
 *  is the resolution `requestFxOverlay` asks the backend to render at; `ripplePreview.ts` upscales
 *  the same mapping back to the real canvas rather than recomputing it (see `drawMirroredRipples`). */

/** The FX-render frame for one composite tick: its size, the screen panel's height fraction, and
 *  the panel-local -> FX-px projection through the active zoom crop. */
export interface FxFrameGeometry {
  fxW: number;
  fxH: number;
  /** Screen-panel height as a fraction of the FX frame height (= `layout.screen[3]`). The
   *  spotlight's radius/feather settings are fractions of the SCREEN, so the caller pre-scales by
   *  this before the backend's `oh * frac` - mirroring the export's `fx_state_at`
   *  (`scene.screen.h / oh`), which is what makes the spotlight track the screen panel in every
   *  layout instead of the whole frame. */
  screenScale: number;
  /** PANEL point (0..1 within the screen panel, the basis the cursor track uses - Rust already
   *  mapped it through the active source span) -> FX-render px. Never actually returns `null`; the
   *  nullable result matches what `requestFxOverlay`/`drawMirroredRipples` already accept so a
   *  future clipped mapping can be dropped in without touching either caller. */
  map: (fx: number, fy: number) => [number, number] | null;
  /** CANVAS point (0..1 of the recorded frame, the basis `ClickSample` uses) -> FX-render px:
   *  `map` composed with the active span's crop rect, the TS mirror of Rust's `to_panel`. Identical
   *  to `map` outside a mid-take display switch; after one, a click recorded on the switched-to
   *  display lands on the cropped picture instead of drifting with the black bars. */
  mapCanvas: (cx: number, cy: number) => [number, number] | null;
}

/** Build the FX-render geometry for one frame. Mirrors `drawPreview`'s whole-frame zoom crop
 *  exactly, just computed at `fxScale` resolution, so a projected spotlight/ripple lands where the
 *  zoomed base frame actually put it. `cw`/`ch` are ROUNDED, matching the export's `coordmap::crop`
 *  - leaving them fractional made the crop a sub-pixel different size, drifting every projected
 *  centre against the export. `lay` is `null` before the layout has loaded, where the same
 *  `0.045`-of-the-short-side padding `drawPreview` falls back to is used. */
export function fxFrameGeometry(
  canvasW: number, canvasH: number,
  lay: PreviewLayout | null,
  cam: { cx: number; cy: number; scale: number },
  fxScale: number,
): FxFrameGeometry {
  const fxW = Math.max(1, Math.round(canvasW * fxScale)), fxH = Math.max(1, Math.round(canvasH * fxScale));
  const pad = Math.min(fxW, fxH) * 0.045;
  const dx = lay ? lay.screen[0] * fxW : pad, dy = lay ? lay.screen[1] * fxH : pad;
  const dw = lay ? lay.screen[2] * fxW : fxW - 2 * pad, dh = lay ? lay.screen[3] * fxH : fxH - 2 * pad;
  const scale = Math.max(cam.scale, 0.01);
  const cw = Math.max(1, Math.round(fxW / scale)), ch = Math.max(1, Math.round(fxH / scale));
  const camPxX = dx + cam.cx * dw, camPxY = dy + cam.cy * dh;
  const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, fxW - cw));
  const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, fxH - ch));
  const src = lay?.src ?? FULL_SRC;
  const map = (fx: number, fy: number): [number, number] => {
    const bx = dx + fx * dw, by = dy + fy * dh; // panel-local point, pre-zoom
    return [(bx - cx0) * fxW / cw, (by - cy0) * fxH / ch];
  };
  return {
    fxW, fxH,
    screenScale: fxH > 0 ? dh / fxH : 1,
    map,
    mapCanvas: (cx: number, cy: number) => { const [px, py] = toPanelFrac(cx, cy, src); return map(px, py); },
  };
}

/** The camera PiP rect for the spotlight's "don't dim the webcam" exclusion, in FX-render px, or
 *  `null` when no camera panel is active this frame. The webcam is a fixed, UNZOOMED overlay drawn
 *  on top (see `drawPreview`), so its rect is the layout fraction applied straight to the FX frame
 *  - deliberately not projected through `map`'s zoom crop like the spotlight centre is.
 *
 *  The `alpha > 0.05` gate mirrors the export's `has_hole` test in `fx_state_at`, NOT `layoutAt`'s
 *  much lower draw threshold: a cross-fading panel passes through the gap between the two, where
 *  the preview would otherwise cut a hole the export never cuts. `camAlpha` is optional on
 *  `PreviewLayout` and treated as `1` when absent, since the static (non-segmented) layout carries
 *  no alpha and a cam rect there is fully shown. */
export function fxCamRect(lay: PreviewLayout | null, fxW: number, fxH: number): FxCamRect {
  if (!lay?.cam || (lay.camAlpha ?? 1) <= 0.05) return null;
  return {
    rect: [lay.cam[0] * fxW, lay.cam[1] * fxH, (lay.cam[0] + lay.cam[2]) * fxW, (lay.cam[1] + lay.cam[3]) * fxH],
    radius: lay.cam[4] * fxW,
  };
}
