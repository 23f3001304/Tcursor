# src/editor/stage/previewDraw.ts

The canvas-drawing primitives `previewCanvas.ts` composites with: one-shot panel compositing, cover-fit drawing and a rounded-rect path. Pure drawing - no layout, no timing, no span or zoom math - split out of `previewCanvas.ts` for its line budget so that file stays the frame it composites.

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
