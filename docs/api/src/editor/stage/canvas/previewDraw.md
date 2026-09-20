# src/editor/stage/canvas/previewDraw.ts

The Canvas2D primitives `drawPreview` composites with: the once-per-panel alpha blit, the cover-crop draw and the rounded-rect path. Split back out of `previewCanvas.ts` - they are the drawing vocabulary, not the frame recipe, and every one of them mirrors a specific piece of the export's shader. `drawScreenPanel` at the end (Batch 4 T7) is the one composite draw here rather than a primitive: the screen panel moved out of `drawPreview` whole, so the clip dissolve is one branch inside one function instead of a second copy of the panel.

## paintPanel

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
- The cursor's panel scale/clip math (`panelFactor`/`panelClipRect`/`contentScale`) is documented in `cursorPanel.ts`, where it's actually defined - pulled out as pure functions so this exact export mirror is unit-tested directly.

## paintPanel

```ts
export function paintPanel(target: CanvasRenderingContext2D, alpha: number, scratch: HTMLCanvasElement,
  w: number, h: number, draw: (c: CanvasRenderingContext2D) => void)
```

Composite one panel ONCE at `alpha`, the way the export's shader does - a single `mix(color, panel, cov * panel_a)` per panel (`gpu/shader.wgsl`).

*Why the scratch layer:* setting `globalAlpha` and then drawing the panel's own layers straight onto `target` composites EACH of them separately, so the shadow backing and the video both land at `alpha` and a panel at alpha 0.5 came out as `0.5*video + 0.25*bg + 0.25*black` - visibly dark through every layout cross-fade, worst at the middle of it, which the export never does. Painting them into a scratch layer and blitting that once is the picture the shader produces (plus the preview's decorative shadow, which now fades WITH its panel instead of as a layer of its own).

Opaque is the overwhelmingly common case and takes the direct path, so the extra full-canvas buffer is only touched on frames a transition is actually running. A missing 2D context falls back to the direct path: an undimmed panel beats no panel.

## coverDraw

```ts
export function coverDraw(ctx: CanvasRenderingContext2D, img: CanvasImageSource, sw: number, sh: number,
  dx: number, dy: number, dw: number, dh: number)
```

Draw `img` to cover the dest rect, centre-cropping the source to the dest aspect. The webcam PiP's framing, mirroring the export's `compositor::cover_rect` / `shader.wgsl`'s `cover_uv`.

## roundRect

```ts
export function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number)
```

Trace a rounded-rect path (radius clamped to half the shorter side), for the fill, clip and stroke of every panel.

## ScreenPanelGeom

```ts
export interface ScreenPanelGeom {
  dx: number; dy: number; dw: number; dh: number; r: number;
  w: number; h: number; alpha: number; src: [number, number, number, number];
}
```

Everything the screen panel needs for one frame, in one value: the panel rect and corner radius in canvas pixels, the canvas size `paintPanel`'s scratch layer is sized to, the layout's interpolated `screenAlpha`, and `src`, the active source span as a fraction of the RECORDED canvas. It is a struct rather than nine parameters because `drawPreview` resolves all nine in one place and hands them straight over.

## ScreenMix

```ts
export interface ScreenMix { video: HTMLVideoElement; alpha: number }
```

The clip dissolve for one frame: the second hidden `<video>`, parked on the outgoing clip's tail, and the INCOMING clip's weight. `null` is the ordinary case and the one every document without a clip transition takes.

## drawScreenPanel

```ts
export function drawScreenPanel(octx: CanvasRenderingContext2D, layer: HTMLCanvasElement,
  g: ScreenPanelGeom, screen: HTMLVideoElement, mix: ScreenMix | null)
```

The screen panel's own draw, lifted out of `drawPreview` (Batch 4 T7) so the dissolve has somewhere to live: the drop shadow and the black backing once through `paintPanel`, then the video through the source span's crop, clipped to the rounded rect. With `mix` at `null` it is the block that used to sit inline, statement for statement, including its two guards (`videoWidth`/`videoHeight` above zero, `alpha` at least 0.004).

**The dissolve.** With a `mix` whose video has decoded (`videoWidth > 0`), the OUTGOING frame is drawn first at full alpha and the incoming one over it at `mix.alpha`, both through the same source rect and into the same destination rect, inside the same clip. That is `alpha * cur + (1 - alpha) * prev` within the panel, which is exactly what `screen_mix::blend_into` computes on the nv12 planes in the export. The shadow and the black backing happen ONCE, under both, because they are the panel and not the picture; and only the SCREEN plane dissolves, matching the export, where the webcam panel and the background are untouched at a clip boundary. A `mix` whose element has not decoded yet draws the incoming frame alone at full alpha, which is the pre-dissolve picture rather than a black flash.

Both elements carry the same `src` attribute (the one screen proxy), so `vw`/`vh` read off the incoming element frame the outgoing one correctly too.
