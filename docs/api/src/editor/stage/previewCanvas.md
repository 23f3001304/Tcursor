# src/editor/stage/previewCanvas.ts

The editor preview's Canvas2D compositor: given the screen `<video>`, the webcam `<video>`, the interpolated camera pose, and the export framing/background, it draws one full preview frame that matches the export's compositing order exactly - background + screen panel are composited onto an offscreen buffer *unzoomed*, then that whole buffer is cropped/resized per the camera (mirroring the export's `coordmap::crop`), so the background pans/zooms in lockstep with the screen instead of a plain crop of the raw recording sitting in a static panel. The webcam PiP and cursor are drawn on top of the zoomed result afterward, at their normal unzoomed size - also matching the export, which composites/projects them after its own crop. Native `drawImage` keeps it at 60fps.

## DrawCam

```ts
export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }
```

The camera pose for one frame, interpolated from the backend `CamSample` track by `camAt`: the zoom `scale`, the zoom centre `cx`/`cy`, and the cursor `curx`/`cury` - all 0..1 fractions of the *screen content* (not the output frame). `drawPreview` converts these into panel-local pixels via the screen rect, then into the whole-frame zoom crop.

## drawPreview

```ts
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: HTMLImageElement | null, clicks: ClickSample[], now: number,
  cursor: DrawCursor | null, offscreen: HTMLCanvasElement, layer: HTMLCanvasElement,
  insetW: number = 1
): void
```

Composites one preview frame onto `ctx` (a `w`x`h` canvas).

### Inputs

- `ctx`, `w`, `h` - the 2D context and canvas size. *Why:* the backing store follows the resolved aspect (`PreviewLayout.canvas`, `Stage.tsx`) and CSS scales that to fit.
- `screen` - the screen proxy `<video>`. *Why:* its current frame is drawn full-size into the (unzoomed) screen panel; the zoom crop is applied afterward to the whole buffer, not to this source.
- `webcam` - the webcam `<video>` or `null`. *Why:* drawn as the PiP panel (cover-fit, rounded) on top of the zoomed result when present and non-empty.
- `cam: DrawCam` - the interpolated camera pose. *Why:* defines the whole-frame crop rectangle (`cw = w/scale`, centred on the panel-local `cx`/`cy`, clamped into the frame) that both the base image and the cursor project through.
- `layout: PreviewLayout | null` - the export framing (screen rect + radius + webcam rect) as fractions. *Why:* so the preview frames the screen/webcam exactly like the export; `null` uses an inset fallback.
- `bg: HTMLImageElement | null` - the decoded export background. *Why:* painted under the screen (onto the offscreen buffer) once loaded, else a gradient placeholder.
- `clicks: ClickSample[]`, `now: number` - the click track and current output time (ms). *Why:* passed through to `drawCursorSprite` for its click-bounce dip animation (ripples themselves are drawn by the backend FX overlay, not here).
- `cursor: DrawCursor | null` - the cursor inputs, or `null` to skip. *Why:* drawn (Enhanced only) at the zoom-projected cursor position with bounce + motion trail, scaled by the screen panel's own size and clipped to its on-screen rect (see `insetW`) - matching the export, which doesn't scale cursor size with the CAMERA zoom either.
- `offscreen: HTMLCanvasElement` - a reusable buffer canvas the caller owns (resized here to `w`x`h` if needed). *Why:* the background+screen composite must exist as one image before the whole-frame crop can read a sub-rect of it; owning this in the caller (`useCompositeLoop`) avoids allocating a new canvas every frame.
- `layer: HTMLCanvasElement` - a second reusable buffer the caller owns, distinct from `offscreen`. *Why:* `paintPanel` (below) composites a part-transparent panel into this and blits it ONCE, so a panel mid-cross-fade is not composited layer-by-layer. Only resized/drawn on the frames a transition is actually running.
- `insetW: number` (fraction of `w`, default `1`) - `LayoutPresets.inset_w`, the export's fixed reference width for the cursor's scale-down (`cursorset::draw`'s `panel` factor; see `cursorPanel.ts`). *Why default 1:* a neutral "no shrink" fallback for the brief window before layout presets have loaded, same spirit as the `layout: null` inset fallback above.

### Returns

`void` - draws directly onto `ctx`.

### Implementation

1. Resolve the screen rect (`dx,dy,dw,dh,r`) from `layout` (or an inset fallback).
2. On `offscreen`: paint the background (image or gradient), then draw the screen video's *full current frame* into the screen rect, clipped to a rounded rect with a drop shadow - through `paintPanel` at the layout's interpolated `screenAlpha`. No zoom crop happens here.
3. Compute the whole-frame zoom crop `(cx0, cy0, cw, ch)` from `cam.scale` and the panel-local `cam.cx`/`cam.cy`, clamped into `[0, w]`x`[0, h]` - the same math as the export's `coordmap::crop`.
4. `ctx.drawImage(offscreen, cx0, cy0, cw, ch, 0, 0, w, h)` - crop+resize the *entire* offscreen buffer onto the visible canvas in one call.
5. Skip the cursor entirely when `screenAlpha < 0.5`, mirroring `cursorset::draw`'s own gate (`if screen.alpha < 0.5 { return; }`) - the export stops drawing the synthetic cursor once the screen panel is more than half faded out, so a cross-fade into `camera_only` must not leave a cursor hanging over a panel that is no longer there. Otherwise: project the cursor's panel-local position through the same crop; derive `panel = panelFactor(dw / w, insetW)` and `clip = panelClipRect({x:dx,y:dy,w:dw,h:dh}, {cx0,cy0,cw,ch}, w, h)` (`cursorPanel.ts`) from the SAME pre-zoom panel rect and crop, and pass both to `drawCursorSprite`.
6. Draw the webcam PiP on `ctx` through `paintPanel` at the layout's interpolated `camAlpha` (rounded rect from `layout.cam`; the bottom-right-circle fallback applies only when there is NO `layout` at all - a layout that exists and simply hides the webcam, `screen_only` or the static `preview_layout` before presets land, must draw nothing rather than that hardcoded bubble) - unzoomed, on top of the zoomed result. When `layout.cam`'s ring width (`cam[5]`, a fraction of output width) is `> 0`, also stroke the export ring/border: a band `ringPx` wide in `ring_color` (`cam[6..9]`, RGB 0..255), traced just inside the panel edge to match `shader.wgsl`/`compositor.rs`'s inside-only SDF band - achieved by stroking a path inset by `ringPx/2` with `lineWidth = ringPx`, so the centered stroke's outer half lands on the true edge and its inner half sits `ringPx` further in.

### paintPanel

```ts
function paintPanel(target: CanvasRenderingContext2D, alpha: number, scratch: HTMLCanvasElement,
  w: number, h: number, draw: (c: CanvasRenderingContext2D) => void): void
```

Composites one panel ONCE at `alpha`, the way the export's shader does - a single `mix(color, panel, cov * panel_a)` per panel (`gpu/shader.wgsl`).

**The bug it fixes.** Setting `globalAlpha` and then drawing a panel's own layers straight onto the target composites EACH of them separately. The screen panel draws a black rounded-rect backing (which carries its drop shadow) and then the video over it; at `alpha 0.5` that came out as `0.5*video + 0.25*bg + 0.25*black`, so every layout cross-fade visibly DARKENED its panels, worst at the middle of the fade - something the export, with one `mix` per panel, never does. The webcam PiP had the same shape of problem across its backing, border stroke and ring.

Painting those layers into a scratch canvas and blitting it once reproduces the shader's result (plus the preview's decorative drop shadow, which now fades *with* its panel instead of as a layer of its own).

`alpha >= 0.999` - the overwhelmingly common case - takes the direct path and never touches `scratch`, so the extra full-canvas buffer only costs anything on frames where a transition is actually running. A `scratch` canvas that yields no 2D context falls back to drawing direct: an undimmed panel beats no panel.

### Notes

- Private helpers in this file: `paintPanel` (above), `coverDraw` (centre-crop cover fit) and `roundRect` (rounded-rect path).
- The FX overlay (spotlight + click ripples) is rendered separately by the backend (`preview_fx.rs`, via `fxOverlay.ts`/`useCompositeLoop.ts`) and blitted on top after this function returns - it is not part of `drawPreview`.
- The cursor type (`DrawCursor`) is documented in `cursorPreview.ts`, where it's actually defined.
- The cursor's panel scale/clip math (`panelFactor`/`panelClipRect`) is documented in `cursorPanel.ts`, where it's actually defined - pulled out as pure functions so this exact export mirror is unit-tested directly.
