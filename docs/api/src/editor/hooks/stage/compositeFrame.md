# src/editor/hooks/stage/compositeFrame.ts

Everything the Stage's rAF loop paints in one tick, split out of `useCompositeLoop.ts`: the camera pose, the frame's layout, the cursor input, `drawPreview`, the mirrored click ripples, the spotlight resolve and overlay blit, the captions and the held exact frame. The loop owns the clock and the media elements; this file owns the picture, and takes everything it needs as arguments so it has no state of its own beyond the scratch object it is handed.

## FrameScratch

```ts
export interface FrameScratch {
  fxOverlayImgRef, fxSeparableRef, fxInflightRef, fxLastTRef, fxWantRef,
  offscreenRef, layerRef, tiltRef
}
```

The draw's persistent state across ticks, as plain `{ current }` boxes so `fxRequestTick` (which takes `RefObject`s) can be handed a subset of it unchanged. Two scratch canvases: `offscreenRef` holds the background+screen base the zoom crop reads a sub-rect of, and `layerRef` is the separate buffer `paintPanel` (`../../stage/canvas/previewDraw.md`) uses to composite a part-transparent panel in one blit - they must not be the same canvas, since the panel draws INTO the base. `tiltRef` is the cursor's motion-tilt filter, advanced once per painted tick and reset by the loop on a discontinuous jump. The five `fx*` refs are the overlay request's cache key, its in-flight gate and the decoded image it landed.

## newFrameScratch

```ts
export function newFrameScratch(): FrameScratch
```

One fresh scratch object. Called once per `useCompositeLoop` (through `useRef`), never per frame - the whole point of the object is that it outlives the tick.

## drawCompositeFrame

```ts
export function drawCompositeFrame(
  ctx: CanvasRenderingContext2D, c: HTMLCanvasElement, sv: HTMLVideoElement,
  r: CompositeLoopRefs, s: FrameScratch,
  play: boolean, t: number, tOut: number, dtOut: number,
): void
```

Paints one frame. `t` is clip time and `tOut` output time (see `useCompositeLoop.md` for why there are two clocks); `dtOut` is the output-clock delta since the last painted tick, `0` across a jump. `play` is this tick's play state, read once by the loop so the exact-frame branch and the loop agree. The whole body is wrapped in one `try`/`catch` that logs in DEV only: a draw that throws must not kill the rAF loop.

### Implementation

1. Resolve the frame's layout (`layoutAt`, falling back to the static `layoutRef`), then hand it to `frameCamLayout` (`src/editor/stage/camera/frameCam.ts`), which derives `live` - the layout-resolved PiP pose - from `baseLayout.cam`'s center + height fraction and resolves the pose to draw the webcam at: `drag ?? camMoveAt(moves, t, live)`, where `drag` is `activeCamDraft(dragPoseRef.current, arrangingRef.current)` - `null` while `arrangingRef` is true, so arrange mode's own draft (which lives in the BASE layout rect `frameCamLayout` receives, not in `dragPoseRef`) isn't outranked by a stale Move-mode drag; see `useCompositeLoop.md`'s `arrangingRef` input and `frameCam.md`. A live Move drag pose wins outright when set and arrange mode isn't active; otherwise the sampled track, which since Task 27 is `null` outside the span the keyframes own and eases to/from `live` at that span's edges (mirrors the Rust `step_camera`/`static_cam_pose`). If the result is non-`null` AND the resolved layout has a camera panel this frame (`cam !== null`), that panel's `cam` tuple is replaced with `overrideCamPanel(baseLayout.cam, cp, c.width, c.height)` (`cameraMoves.ts`) so the webcam PiP's rect, radius, AND ring width follow the drag/track exactly like the export's `step_camera`/`override_camera`; ring color and alpha are left as resolved. An empty track (or a playhead outside the keyframes' span) with no drag in progress falls through to the smart webcam-on-zoom action, and a frame with no camera panel (e.g. mid `screen_only` segment) leaves the layout untouched.
2. Build the cursor input - the live `CursorSettings` plus `kindsRef` and every map on `spritesRef` (`sprites`/`hots`/`canvasH`, `captured`, the decoded real OS-cursor layer that makes `drawCursorSprite` composite the recorded bitmap instead of a synthetic sprite, AND `busy`/`busyFrames`, pack format v2's animated busy state) - then advance the motion-tilt filter once for this tick (`tiltFromCam`, `../../stage/cursor/cursorTilt.ts`) from `cam.curx`/`cam.cury` and the resolved screen panel's own aspect, writing the result into `cur.tiltDeg`. *Why the filter's state lives in the scratch object:* it is advanced exactly once per painted tick, the same shape as `trailRef` beside it, so it has to outlive the call while nothing else may touch it. *Why the panel's aspect:* `cury` is a fraction of the panel's HEIGHT while `curx` is a fraction of its width, and the filter needs both in one unit. Then call `drawPreview` (background + screen + webcam + cursor, no FX) with the (possibly overridden) layout, into an offscreen buffer it owns (`offscreenRef`, created lazily, resized to the canvas size by `drawPreview` itself).
3. Everything else the tick paints - the mirrored click ripples, the spotlight resolve, the FX-overlay blit, the captions, and the FX-overlay request/response bookkeeping - is `drawOverlays(ctx, c, r, s, t, tOut, frameLayout, cam)` (`../../stage/fx/overlayDraw.ts`), called right after `drawPreview` with this tick's already-resolved `frameLayout` and `cam`. See `../../stage/fx/overlayDraw.md` for that sequence, the throttle/downscale and spotlight-fade rationale, and the two seams Batch 2's mask/grade/text calls use.
4. Last, on a PAUSED tick only: if `exactRef` holds a frame whose key equals `exactKey(t, editGenRef.current)`, draw it over the whole canvas - the export's own frame for this instant (owner ruling 2026-09-14: the export is the reference). A frame from a scrubbed-past instant or an older doc never matches, and playing never draws it.

### Notes

- Owns TWO scratch canvases for `drawPreview`, both lazily created and reused for the life of the loop: `offscreenRef` holds the background+screen base the zoom crop reads a sub-rect of, and `layerRef` is the separate buffer `paintPanel` (`stage/canvas/previewCanvas.md`) uses to composite a part-transparent panel in one blit. They must not be the same canvas - the panel draws INTO the base.
- The throttle/downscale rationale for the FX-overlay request, the spotlight-fade-at-blit-time fix, the FX coordinate mapping, and the `camRect`/`layoutAt` threshold gap all now live in `../../stage/fx/overlayDraw.md`, next to the code that does them.
