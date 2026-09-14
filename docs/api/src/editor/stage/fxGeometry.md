# src/editor/stage/fxGeometry.ts

The pure geometry the FX overlay needs for one composite tick, extracted from `useCompositeLoop`'s rAF body so it is unit-testable without a canvas (and so that hook stays inside the file-size budget) - the same extraction pattern as `useStageInvalidation.ts` / `pointerGesture.ts`.

Everything here works in the **downscaled FX-render space** (`fxW` x `fxH` = the canvas times `fxScale`), the resolution `requestFxOverlay` asks the backend to render at. `ripplePreview.ts`'s `drawMirroredRipples` upscales this same mapping back to the real canvas rather than recomputing an equivalent full-resolution one - the mapping is linear/homogeneous in `fxW`/`fxH`, so scaling its output by `(canvasW/fxW, canvasH/fxH)` is exactly that recomputation.

## FxFrameGeometry

```ts
export interface FxFrameGeometry {
  fxW: number;
  fxH: number;
  screenScale: number;
  map: (fx: number, fy: number) => [number, number] | null;
  mapCanvas: (cx: number, cy: number) => [number, number] | null;
}
```

The two mappings differ only once a mid-take display switch has cropped the picture, and confusing them is the bug they exist to prevent:

- `map` takes a PANEL point (0..1 within the screen panel) - what the cursor track carries, since Rust already mapped it through the active source span.
- `mapCanvas` takes a CANVAS point (0..1 of the recorded frame) - what a `ClickSample` carries - and is `map` composed with the active span's crop rect (`sourceSpans.ts`'s `toPanelFrac`), the TS mirror of Rust's `to_panel`. Pass it wherever click positions are projected: `drawMirroredRipples` and `requestFxOverlay`'s hits.

- `fxW` / `fxH` - the FX render size in px, floored at 1 so a degenerate canvas can never produce a zero-sized request.
- `screenScale` - the screen panel's height as a fraction of the FX frame height (= `layout.screen[3]`). The spotlight's `radius`/`feather` settings are fractions of the **screen**, so the caller pre-scales by this before the backend's `oh * frac`, mirroring the export's `fx_state_at` (`scene.screen.h / oh`). That is what makes the spotlight track the screen panel in every layout instead of the whole frame.
- `map` - screen-content point (0..1 within the screen panel, the basis `ClickSample` and the cursor track use) -> FX-render px, through the active zoom crop. Never actually returns `null`; the nullable result matches what `requestFxOverlay` / `drawMirroredRipples` already accept, so a future clipped mapping can be dropped in without touching either caller.

## fxFrameGeometry

```ts
export function fxFrameGeometry(
  canvasW: number, canvasH: number,
  lay: PreviewLayout | null,
  cam: { cx: number; cy: number; scale: number },
  fxScale: number,
): FxFrameGeometry
```

Builds that geometry for one frame. Mirrors `drawPreview`'s whole-frame zoom crop exactly, just computed at `fxScale` resolution, so a projected spotlight centre or ripple lands where the zoomed base frame actually put it.

### Implementation

1. `fxW`/`fxH` = the canvas times `fxScale`, rounded, floored at 1.
2. The screen panel's rect in FX px comes from `lay.screen`; when `lay` is `null` (before the layout has loaded) it falls back to the same `0.045`-of-the-short-side padding `drawPreview` uses.
3. The crop is `fxW/scale` x `fxH/scale`, centred on `(cam.cx, cam.cy)` **within the panel** and clamped to stay inside the frame. `cw`/`ch` are **rounded**, matching the export's `coordmap::crop` - leaving them fractional made the crop a sub-pixel different size, so every projected centre drifted slightly against the export. `scale` is floored at `0.01`, so a degenerate zoom cannot divide by zero.
4. `map` applies the panel rect, then the crop, in that order.

### Behaviors

- `renders at fxScale and reports the screen panel's height fraction`
- `maps panel-local 0..1 onto the screen panel's rect when unzoomed`
- `projects through the zoom crop, magnifying around the camera centre` - the camera centre is a fixed point and offsets scale by the zoom factor.
- `clamps the crop inside the frame instead of sampling off its edge`
- `falls back to a padded full frame before the layout has loaded`
- `treats a degenerate zoom scale as no zoom rather than dividing by zero`

## fxCamRect

```ts
export function fxCamRect(lay: PreviewLayout | null, fxW: number, fxH: number): FxCamRect
```

The camera PiP rect for the spotlight's "don't dim the webcam" exclusion, in FX-render px, or `null` when no camera panel is active this frame.

The webcam is a fixed, **unzoomed** overlay drawn on top (see `drawPreview` / `previewCanvas.ts`), so its rect is the layout fraction applied straight to the FX frame - deliberately *not* projected through `map`'s zoom crop the way the spotlight centre is.

The `alpha > 0.05` gate mirrors the export's `has_hole` test in `fx_state_at`, **not** `layoutAt`'s much lower draw threshold: a cross-fading panel passes through the gap between the two, where the preview would otherwise cut a hole the export never cuts. `camAlpha` is optional on `PreviewLayout` and treated as `1` when absent, since the static (non-segmented) layout carries no alpha and a cam rect there is fully shown.

### Behaviors

- `is null when the layout has no camera panel`
- `converts the layout fractions to FX px WITHOUT the zoom crop (the PiP is unzoomed)`
- `gates on the export's has_hole threshold, not layoutAt's lower draw threshold`
- `treats a missing camAlpha as fully shown (the static, non-segmented layout)`

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - once per composite tick, for the ripple draw, the FX-overlay request, and the cursor projection.
