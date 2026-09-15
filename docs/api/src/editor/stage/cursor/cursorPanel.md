# src/editor/stage/cursor/cursorPanel.ts

The export's synthetic cursor scales AND clips to the screen panel (`cursorset.rs`: "scaled to the panel size ... and clipped to the panel's on-screen rect so it never spills onto the background or the webcam"). Pulled out of `previewCanvas.ts` as pure ratio/projection math - no canvas, no DOM - so this exact mirror is unit-tested directly rather than eyeballed on screen. Same extraction pattern as `fxGeometry.ts` / `pointerGesture.ts`.

## panelFactor

```ts
export function panelFactor(screenW: number, insetW: number): number
```

The cursor's scale-down factor for a screen panel of width `screenW`, against the export's fixed reference inset width `insetW` (`LayoutPresets.inset_w` - a baseline independent of the active preset/arrangement, NOT the panel's own preset rect).

### Inputs

- `screenW: number` - the active screen panel's own width.
- `insetW: number` - the export's reference inset width (`LayoutPresets.inset_w`). Both args must be the same unit (canvas px, or a fraction of the canvas - it's a pure ratio, so either works); `previewCanvas.ts` passes fractions since that's what `PreviewLayout.screen`/`LayoutPresets.inset_w` already are.

### Returns

`number` - `(screenW / max(insetW, 1e-6)).clamp(0.1, 1.0)`, mirroring `cursorset.rs`'s `(screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0)` exactly, except the divide-by-zero floor is a tiny epsilon rather than "1 output px" (which has no fixed meaning in fraction units) - it only ever matters for a degenerate zero `insetW`, never a real one.

### Used by

- `src/editor/stage/canvas/previewCanvas.ts` - `drawPreview` computes `panelFactor(dw / w, insetW)` from the screen panel's own pre-zoom width and passes it to `drawCursorSprite`.

## ZoomCrop

```ts
export interface ZoomCrop { cx0: number; cy0: number; cw: number; ch: number }
```

The zoom crop `previewCanvas.ts` resizes the whole base frame through (its `cx0`/`cy0`/`cw`/`ch`) - same shape as the export's `coordmap::crop`.

## panelClipRect

```ts
export function panelClipRect(
  rect: { x: number; y: number; w: number; h: number },
  crop: ZoomCrop,
  canvasW: number, canvasH: number,
): [number, number, number, number]
```

Projects `rect` (pre-zoom canvas px, the offscreen base-frame space `drawPreview` composites into - e.g. the screen panel's own `dx`/`dy`/`dw`/`dh`) through `crop` the same way a point is mapped to its on-screen position, then clamps to the canvas bounds. Mirrors `cursorset.rs`'s `project_rect` (in turn `coordmap::project` on each corner + `coordmap::crop`) - the box the export confines the cursor AND its motion trail to, so neither ever spills onto the background or the webcam.

### Inputs

- `rect` - the panel's own pre-zoom rect.
- `crop: ZoomCrop` - the active zoom crop.
- `canvasW`, `canvasH` - the canvas size to clamp into.

### Returns

`[number, number, number, number]` - post-zoom canvas px `[x0, y0, x1, y1]`. Deliberately NOT normalized when the panel falls entirely outside the crop (`x0` can end up past `x1`) - exactly like the Rust tuple, which relies on its caller's `ox_start >= ox_end` no-op check rather than reordering; `drawCursorSprite` carries the same guard before it draws anything.

### Used by

- `src/editor/stage/canvas/previewCanvas.ts` - `drawPreview` computes `panelClipRect({x:dx,y:dy,w:dw,h:dh}, {cx0,cy0,cw,ch}, w, h)` from the same panel rect and crop `drawPreview` already has, and passes it to `drawCursorSprite`.
- `src/editor/stage/cursor/cursorPreview.ts` - `drawCursorSprite` no-ops when the result is empty/inverted, and clips the trail + main blit to it otherwise.

## contentScale

```ts
export function contentScale(panel: number, insetPx: number, srcW: number): number
```

`srcW` is the source width the panel actually SHOWS - the whole recorded frame normally, but only the active source span's slice of it after a mid-take display switch, so the caller scales it by `PreviewLayout.src`'s width. Rust does the same, passing `Scene.src.w` into `captured::content_scale` where it used to pass the canvas width; the full width after a switch would size the cursor against pixels the panel is not drawing.

Canvas px per SOURCE px for the screen panel - the scale that keeps the CAPTURED cursor at its true size relative to the screen content. The TS mirror of Rust `export::cursor::captured::content_scale`, pinned by the same numbers on both sides.

### Inputs

- `panel: number` - `panelFactor`'s result (0.1..1.0).
- `insetPx: number` - the reference inset width in **canvas px**. `previewCanvas.ts` passes `insetW * w`: unlike `panelFactor`, this is not a pure ratio - `srcW` is in real pixels, so the two must share a unit, and `LayoutPresets.inset_w` is a canvas fraction.
- `srcW: number` - the recorded video's own width in pixels, from `CursorLayerDto.src_w` (carried on `CapturedLayer.srcW`). NOT `video.videoWidth`, which is the downscaled preview proxy.

### Returns

`panel * insetPx / srcW`. Since `panel * insetPx` IS the screen panel's own canvas width, this is `panelWidth / srcW` - the same ratio the canvas draws the screen content at - with the panel shrink applied exactly ONCE.

A non-positive `srcW` (the backend could not probe the video) falls back to `panel` alone, i.e. the pre-content-scale behavior, rather than collapsing the cursor to nothing on a divide-by-zero.

### Behaviors

- `leaves a source the same size as its panel untouched` / `halves a 2x source (a 4K take on a 1080p canvas)` / `grows an upscaled source with its content` - the ratio itself.
- `applies a shrunk panel once, not squared` - 0.5 panel gives 0.5 and 0.25, never 0.25 and 0.0625.
- `falls back to the panel factor alone when the source width is unknown` - `srcW` 0 or negative.
