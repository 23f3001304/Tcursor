# src/editor/stage/fx/overlayDraw.ts

Everything drawn ON TOP of the composited base frame, in order: the mirrored click ripples, the spotlight resolve and its cached FX-overlay blit, the captions, then the FX-overlay request/response bookkeeping that primes the NEXT tick's blit. Split out of `compositeFrame.ts`'s `drawCompositeFrame` (editor-parity foundations, task 9) so it is the one small file Batch 2's mask/grade/text agents each add one call to, rather than three agents converging on the same tick function. `drawCompositeFrame` calls it once per tick, right after `drawPreview` paints the background/screen/webcam/cursor and before the paused tick's held exact frame.

The seams Batch 2 uses, mirroring the export's ruled order (`render/fx_step.md`): Batch 2a's mask call goes BEFORE the cached FX-overlay blit, since masks precede the FX pass in `fx_pass`, and it must land UNDER the cursor to match the export - the preview paints the cursor in `drawPreview`, before this file runs at all, so how a mask gets beneath it is Batch 2a's design problem, not a free consequence of call order. Batch 2b's grade covers the composited PICTURE only, never the overlays drawn here. Batch 2c's text goes after the FX blit and before `drawCaptions`. The export is the reference: where the two cannot be made to agree by ordering alone, the preview is what changes.

## drawOverlays

```ts
export function drawOverlays(
  ctx: CanvasRenderingContext2D,
  c: HTMLCanvasElement,
  r: CompositeLoopRefs,
  s: FrameScratch,
  t: number,
  tOut: number,
  frameLayout: ReturnType<typeof frameCamLayout>,
  cam: Cam,
): void
```

Paints the overlay stack for one tick onto the same canvas `drawPreview` just painted the base frame into.

### Inputs

- `ctx`, `c` - the stage canvas and its 2D context, mid-tick.
- `r`, `s` - the same `CompositeLoopRefs` and `FrameScratch` `drawCompositeFrame` was called with. `r.clickfxRef.current` is read here (as `cf`) rather than passed through by the caller.
- `t`, `tOut` - clip time and output time; see `../../hooks/stage/compositeFrame.md` for why both exist.
- `frameLayout`, `cam` - this tick's already-resolved layout and camera pose (`frameCamLayout`'s return value and `camAt`'s result), computed once by `drawCompositeFrame` and reused here rather than recomputed.

### Returns

`void`. Draws directly onto `ctx`; nothing is returned.

### Implementation

1. Compute the FX-render geometry (`fxFrameGeometry`, `./fxGeometry.ts` - `fxW`/`fxH`, `screenScale`, and the `mapFn` projection; see Notes) once, THEN call `drawMirroredRipples` (`./ripplePreview.ts`), passing `cf.enabled` (`clickfx.enabled`) straight through, to draw this tick's active click ripples straight on the real canvas. `drawMirroredRipples` checks `enabled` FIRST, same as `requestFxOverlay` below. Drawn BEFORE the overlay blit (next step), so an active spotlight's dim (still overlay-rendered) composites on top of it like every other preview element that isn't the ripple itself.
2. Call `resolveSpotlight` **exactly once** per tick (mutates the persistent `spotSimRef` in place - a second call at the same time would double-advance the same stateful transition), then `spotAlphaPlan` (`./spotlightPreview.ts`) with `overlayNeedsClicks(cf.style)`.
3. Blit the cached FX-overlay image (`fxOverlayImgRef`) on top, if one exists and has decoded. It carries the spotlight, plus click hits too when the active style is one `ripplePreview.ts` does NOT mirror (`overlayNeedsClicks` - Glow/Neon/Particles; see `fxOverlay.md`). The blit's `ctx.globalAlpha` is `plan.drawAlpha` when the CACHED image is separable (`fxSeparableRef` - see the spotlight-fade note below), else `1`; it is wrapped in `save`/`restore`, and skipped entirely at `0`.
4. Draw the spoken captions: `drawCaptions(ctx, c.width, c.height, captionsRef.current, capStyleRef.current, accentRef.current, tOut)` - after the FX blit, the order `composite_at` paints them in. Note `tOut`, not `t`: the caption track is a doc region list and `remapDoc` has already moved it (and each caption's own word timings) onto the output clock. See `./captionDraw.md`.
5. `fxRequestTick` (`../../hooks/stage/fxRequestTick.ts`) computes this tick's FX cache key (bucketed to `FX_BUCKET_MS`, ~25fps) and, if it differs from the last APPLIED key and no request is already in flight, fires `requestFxOverlay` at `FX_SCALE` (0.5x) resolution for the NEXT tick's blit. See that file's own doc for the cache-key/response-latching detail - the throttle/downscale rationale below is about the request this step makes.

### Notes

- **Why the throttle/downscale exist:** the FX overlay is a full Tauri IPC round-trip (shader render, a per-pixel alpha-reconstruct pass, PNG encode, base64, a JS `Image` decode). The spotlight tracks the cursor, which moves on nearly every frame during playback, so a naive "request whenever the projected position changes" fires that round-trip ~60x/sec - this was the actual cause of a reported preview stutter. `FX_BUCKET_MS` (`fxRequestTick.ts`) caps the request cadence; `FX_SCALE` (0.5, this file) cuts the backend's per-pixel/PNG/decode cost by rendering at half linear resolution (the effects are soft gradients/ripples, invisible when the canvas upscales them on blit). Only one FX request is ever in flight at a time (the `fxInflightRef` gate), so when the round-trip takes longer than `FX_BUCKET_MS`, the effective update rate is bounded by that round-trip time, not the bucket - by design, not a bug.
- **Why the spotlight's fade is applied here, not by the backend.** Every other effect is drawn per-frame in TS at 60fps; the spotlight is a backend-rendered PNG. Requesting it at `FX_BUCKET_MS` cadence with single-flight gating means a 250ms fade got a handful of overlay updates at best, and none at all whenever one request outlived the fade - so it popped on and off instead of fading, the one effect in the editor with no visible entry or exit animation. `spotAlphaPlan` (`./spotlightPreview.ts`) fixes that by asking the backend for the spotlight's *alpha-independent* appearance (`requestAlpha: 1`) and multiplying the live alpha in at blit time (step 3), which is exact for the modes in `ALPHA_LINEAR_SPOT_MODES` and costs no IPC. It falls back to the previous request-at-live-alpha behaviour whenever the overlay is not spotlight-only (an unmirrored click style bakes its rings into the same PNG) or the mode's alpha response is not proportional. `fxSeparableRef` is a SEPARATE ref from `fxOverlayImgRef` on purpose: it must describe the cached IMAGE, since a response can land a frame or two after the plan it was requested under has moved on.
- The FX-overlay coordinate mapping (`mapFn`, and the `(cx0,cy0,cw,ch)` crop it projects through) lives in `./fxGeometry.ts` and mirrors `drawPreview`'s whole-frame zoom crop exactly, just computed at `FX_SCALE` resolution - so spotlight positions land exactly where the zoomed base frame puts them. `drawMirroredRipples` reuses this SAME `mapFn` for ripple positions too, rather than computing an equivalent full-resolution mapping a second time. See `fxGeometry.md` for the rounding and clamping details.
- The `camRect` handed to `requestFxOverlay` (the spotlight's "don't dim the webcam" hole, computed inside `fxRequestTick`) is gated at `camAlpha > 0.05`, matching the export's `has_hole` test in `fx_state_at` - deliberately **not** the `0.004` threshold `layoutAt` uses to decide whether to *draw* the camera panel. A cross-fading panel passes through the gap between those two thresholds, where the preview would otherwise cut a hole the export never cuts.
- The FX math only ever reads the layout's `screen` rect (`lay.screen`), never `cam` - so the `camera_moves` override (which only ever touches `cam`) has no effect on spotlight/click/cursor positioning; it only changes where `drawPreview` draws the webcam PiP.

### Used by

- `src/editor/hooks/stage/compositeFrame.ts` - `drawCompositeFrame` calls it once per tick, after `drawPreview` and before the held exact frame.
