# src/editor/stage/canvas/previewCanvas.ts

The editor preview's Canvas2D compositor: given the screen `<video>`, the webcam `<video>`, the interpolated camera pose, and the export framing/background, `drawPreview` here and `drawCursorLayer` in the sibling `cursorLayer.ts` together draw one full preview frame that matches the export's compositing order exactly - background + screen panel are composited onto an offscreen buffer *unzoomed*, then that whole buffer is cropped/resized per the camera (mirroring the export's `coordmap::crop`), so the background pans/zooms in lockstep with the screen instead of a plain crop of the raw recording sitting in a static panel. The webcam PiP is drawn on top of the zoomed result next, at its normal unzoomed size, then the cursor last, by `drawCursorLayer` - the same screen/webcam/cursor order the export composites in, with the seam between the panels and the cursor (`PreviewGeom`, below) open for Batch 2's mask and grade. Native `drawImage` keeps it at 60fps.

**The colours here are frame pixels, not chrome, so they do NOT follow the editor's theme.** The panel drop shadow, the black backing each panel is drawn on before its video, and the 2px white edge on the webcam PiP are all part of the picture the export produces; a preview that lightened them under the light theme would stop matching what gets written to the file. They are the one place in the editor a literal colour is correct, and the reason `themeTokens.test.ts`'s "no raw hex" scan covers stylesheets rather than this file.

## DrawCam

**Time remap.** `drawPreview` gained an optional last argument `bgTimeMs`: a video background runs on the OUTPUT clock (the exporter feeds it one frame per output frame), so the composite loop passes `tOut` for it while `now` stays clip time for the cursor, the clicks and the trail. Omitted, `now` is used for both, as before.

```ts
export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }
```

The camera pose for one frame, interpolated from the backend `CamSample` track by `camAt`: the zoom `scale`, the zoom centre `cx`/`cy`, and the cursor `curx`/`cury` - all 0..1 fractions of the *screen content* (not the output frame). `drawPreview` converts these into panel-local pixels via the screen rect, then into the whole-frame zoom crop.

## PreviewGeom

```ts
export interface PreviewGeom {
  panel: [number, number, number, number];
  crop: [number, number, number, number];
  src: [number, number, number, number];
  screenAlpha: number;
  hasVideo: boolean;
}
```

The geometry `drawPreview` used to draw the cursor with, handed back across the seam instead: `panel` (`[dx, dy, dw, dh]`) is the screen rect and `crop` (`[cx0, cy0, cw, ch]`) the whole-frame zoom crop, both in visible canvas pixels - the same `w`x`h` space `ctx` is drawn into. `src` is the opposite unit, `layout?.src ?? FULL_SRC`: a `[x, y, w, h]` fraction of the *recorded* canvas, the active source span applied to the screen video's own dimensions. `screenAlpha` and `hasVideo` (`vw > 0 && vh > 0`) are the two conditions `drawCursorLayer` re-checks before it draws, carried across the seam rather than read a second time from `layout`/`screen` - so whatever a mask or grade does between the two calls, the cursor still gates on the frame `drawPreview` actually painted.

## drawPreview

```ts
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: StageBgState | null, now: number,
  offscreen: HTMLCanvasElement, layer: HTMLCanvasElement,
  bgTimeMs?: number, mix: ScreenMix | null = null
): PreviewGeom | null
```

The screen video is drawn through the active source span's crop rect (`PreviewLayout.src`, fractions of the recorded canvas applied to the proxy's own dimensions), so the black bars a mid-take display switch baked into the recording are cropped away here exactly as the export's compositor crops them (`Scene.src`).

`paintPanel`, `coverDraw` and `roundRect` are the canvas-drawing primitives below - pure drawing, no layout, no timing, no span or zoom math.

Composites the background, the screen panel and the webcam panel onto `ctx` (a `w`x`h` canvas) and returns the `PreviewGeom` that frame used - it no longer draws the cursor, which now lives in the sibling module `cursorLayer.ts` (`drawCursorLayer`, `cursorLayer.md`). **The seam** is the gap this return brackets against that call: `drawCompositeFrame` (`hooks/stage/compositeFrame.ts`) holds the returned geometry across it, so Batch 2's `drawMasks` and `gradeCanvas` calls, inserted there, see the same panels and crop the cursor will.

### Inputs

- `ctx`, `w`, `h` - the 2D context and canvas size. *Why:* the backing store follows the resolved aspect (`PreviewLayout.canvas`, `Stage.tsx`) and CSS scales that to fit.
- `screen` - the screen proxy `<video>`. *Why:* its current frame is drawn full-size into the (unzoomed) screen panel; the zoom crop is applied afterward to the whole buffer, not to this source.
- `webcam` - the webcam `<video>` or `null`. *Why:* drawn as the PiP panel (cover-fit, rounded) on top of the zoomed result when present and non-empty.
- `cam: DrawCam` - the interpolated camera pose. *Why:* `scale` and the panel-local `cx`/`cy` define the whole-frame crop rectangle (`cw = w/scale`, clamped into the frame) returned as `PreviewGeom.crop`. `curx`/`cury` pass through unread; only `drawCursorLayer` projects the cursor with them.
- `layout: PreviewLayout | null` - the export framing (screen rect + radius + webcam rect) as fractions. *Why:* so the preview frames the screen/webcam exactly like the export; `null` uses an inset fallback.
- `bg: StageBgState | null` - the whole background state (`stage/canvas/stageBg.ts`): the decoded export background, plus the `<video>` or decoded GIF frames when the doc's background is an imported moving asset. *Why:* painted under the screen onto the offscreen buffer by `drawBackground`, which blits the still PNG (already cover-fitted, blurred and dimmed by Rust - nothing to redo here) or draws this frame of the moving asset cover-fitted with the dim over it. A gradient placeholder until something has loaded.

  *Why a state object and not an image:* it replaced a bare `HTMLImageElement`, so this function's argument count - and `Stage`'s and `useCompositeLoop`'s prop counts - did not grow when video backgrounds landed.

  Preview/export parity for a MOVING background is "the same frame within one output frame", not pixel-identical: this side resamples through the browser's decoder, the export through ffmpeg's `-r`.
- `now: number` - current output time (ms). *Why:* the background's time when `bgTimeMs` is omitted (see `DrawCam`'s note above). The click track and the cursor read it too, but from `drawCursorLayer` now, not here.
- `offscreen: HTMLCanvasElement` - a reusable buffer canvas the caller owns (resized here to `w`x`h` if needed). *Why:* the background+screen composite must exist as one image before the whole-frame crop can read a sub-rect of it; owning this in the caller (`useCompositeLoop`) avoids allocating a new canvas every frame.
- `layer: HTMLCanvasElement` - a second reusable buffer the caller owns, distinct from `offscreen`. *Why:* `paintPanel` (below) composites a part-transparent panel into this and blits it ONCE, so a panel mid-cross-fade is not composited layer-by-layer. Only resized/drawn on the frames a transition is actually running.
- `mix: ScreenMix | null` (Batch 4 T7) - the clip dissolve for this frame: the second hidden `<video>` parked on the outgoing clip's tail and the incoming clip's weight, or `null`. Passed straight to `drawScreenPanel` and read nowhere else here, so a document with no clip transition draws exactly the pixels it drew before. The caller (`compositeFrame.ts`) builds it from `clipMixAt` at the same `tOut` it hands `bgTimeMs`; see `../clips/clipDissolve.md`.

### Returns

A `PreviewGeom` (see above), or `null` when `offscreen` has no 2D context - the one failure case, checked first and before anything is drawn.

### Implementation

1. Resolve the screen rect (`dx,dy,dw,dh,r`) from `layout` (or an inset fallback).
2. On `offscreen`: paint the background (image or gradient), then `drawScreenPanel` (`previewDraw.md`) draws the screen video's *full current frame* into the screen rect, clipped to a rounded rect with a drop shadow - through `paintPanel` at the layout's interpolated `screenAlpha`, and with the outgoing clip's frame under it when `mix` is non-`null`. No zoom crop happens here. *Why the panel is a call and not the block it used to be:* the dissolve needs two draws inside one clip, and a second copy of the shadow, the backing and the crop arithmetic in a dissolve-only branch is the way those two drift apart.
3. Compute the whole-frame zoom crop `(cx0, cy0, cw, ch)` from `cam.scale` and the panel-local `cam.cx`/`cam.cy`, clamped into `[0, w]`x`[0, h]` - the same math as the export's `coordmap::crop`.
4. `ctx.drawImage(offscreen, cx0, cy0, cw, ch, 0, 0, w, h)` - crop+resize the *entire* offscreen buffer onto the visible canvas in one call.
5. Draw the webcam PiP on `ctx` through `paintPanel` at the layout's interpolated `camAlpha` (rounded rect from `layout.cam`; the bottom-right-circle fallback applies only when there is NO `layout` at all - a layout that exists and simply hides the webcam, `screen_only` or the static `preview_layout` before presets land, must draw nothing rather than that hardcoded bubble) - unzoomed, on top of the zoomed result. When `layout.cam`'s ring width (`cam[5]`, a fraction of output width) is `> 0`, also stroke the export ring/border: a band `ringPx` wide in `ring_color` (`cam[6..9]`, RGB 0..255), traced just inside the panel edge to match `shader.wgsl`/`compositor.rs`'s inside-only SDF band - achieved by stroking a path inset by `ringPx/2` with `lineWidth = ringPx`, so the centered stroke's outer half lands on the true edge and its inner half sits `ringPx` further in.
6. Return the `PreviewGeom`: `panel`, `crop`, `src`, `screenAlpha` and `hasVideo` as resolved above.

#
- The drawing primitives it is built from (`paintPanel`, `coverDraw`, `roundRect`) live in `previewDraw.ts`, and are re-exported from here so importers keep one entry point. `drawScreenPanel` and the `ScreenMix` type join them for the same reason.
- `drawCursorLayer`, the cursor's own layer, moved out to the sibling `cursorLayer.ts` (`cursorLayer.md`) so the seam between the panels and the cursor has a real gap for Batch 2's mask and grade to land in.
