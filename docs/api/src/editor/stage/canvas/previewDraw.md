# src/editor/stage/canvas/previewDraw.ts

The three Canvas2D primitives `drawPreview` composites with: the once-per-panel alpha blit, the cover-crop draw and the rounded-rect path. Split back out of `previewCanvas.ts` - they are the drawing vocabulary, not the frame recipe, and every one of them mirrors a specific piece of the export's shader.

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
