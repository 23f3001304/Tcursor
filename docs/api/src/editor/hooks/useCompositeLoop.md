# src/editor/hooks/useCompositeLoop.ts

The Stage's single `requestAnimationFrame` loop, extracted from `Stage.tsx` so that file stays under the size limit. Reads the live time, keeps the webcam/audio roughly synced to the screen video, composites the base frame via `drawPreview`, blits the cached backend FX overlay on top, and throttles/downscales fresh FX-overlay requests.

## useCompositeLoop

```ts
export function useCompositeLoop({
  screenRef, webcamRef, audioRef, canvasRef,
  playRef, timeRef, onTimeRef,
  trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, cameraMovesRef, dragPoseRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  spritesRef, trailRef, dirtyRef, bgImgRef,
}: { /* all live values as RefObjects, mirrored by Stage.tsx / useSyncRefs */ }): void
```

### Inputs

All inputs are `RefObject`s (not plain values) so the one long-lived `rAF` loop always reads current state without re-subscribing - `Stage.tsx` owns the refs (via `useSyncRefs` for the shared ones) and this hook only reads them each tick. See `Stage.tsx` for what each one holds; `dirtyRef` and `bgImgRef` are Stage's own refs (paused-frame dirty flag, decoded background image).

- `dragPoseRef: RefObject<CamPose | null>` - set by `Stage`'s Move-mode drag handle while the user is mid-drag; `null` the rest of the time. When non-`null` it takes precedence over `camMoveAt(cameraMovesRef.current, t)` entirely, so the PiP tracks the live pointer position every frame without writing to the backend - the drag only commits a real `camera_moves` keyframe on pointer-up.

### Returns

`void` - side-effect only; starts the loop on mount, cancels it on unmount.

### Implementation

1. Each tick: if paused and nothing is `dirtyRef`, skip entirely (the idle/interaction-lag fix - an open-but-idle editor shouldn't burn 60fps redrawing a static frame).
2. Read the time (`sv.currentTime` while playing, else `timeRef`), throttle the `onTime` report to ~16fps, nudge webcam/audio back in sync if they've drifted.
3. Resolve the frame's layout (`layoutAt`, falling back to the static `layoutRef`), then hand it to `frameCamLayout` (`src/editor/stage/frameCam.ts`), which derives `live` - the layout-resolved PiP pose - from `baseLayout.cam`'s center + height fraction and resolves the pose to draw the webcam at: `dragPoseRef.current ?? camMoveAt(moves, t, live)` - a live drag pose wins outright when set; otherwise the sampled track, which since Task 27 is `null` outside the span the keyframes own and eases to/from `live` at that span's edges (mirrors the Rust `step_camera`/`static_cam_pose`). If the result is non-`null` AND the resolved layout has a camera panel this frame (`cam !== null`), that panel's `cam` tuple is replaced with `overrideCamPanel(baseLayout.cam, cp, c.width, c.height)` (`cameraMoves.ts`) so the webcam PiP's rect, radius, AND ring width follow the drag/track exactly like the export's `step_camera`/`override_camera`; ring color and alpha are left as resolved. An empty track (or a playhead outside the keyframes' span) with no drag in progress falls through to the smart webcam-on-zoom action, and a frame with no camera panel (e.g. mid `screen_only` segment) leaves the layout untouched.
4. Build the cursor input and call `drawPreview` (background + screen + webcam + cursor, no FX) with the (possibly overridden) layout, into an offscreen buffer it owns (`offscreenRef`, created lazily, resized to the canvas size by `drawPreview` itself).
5. Blit the cached FX-overlay image (`fxOverlayImgRef`) on top, if one exists and has decoded.
6. Compute a cache key (`fxCacheKey`, `fxCacheKey.ts`) from the zoom-projected cursor position, spotlight params, click track, and click-FX params, bucketed to `FX_BUCKET_MS` (~25fps, `timeBucket`) - **not** every rAF tick - and record it in `fxWantRef` unconditionally (whether or not a request actually fires this tick). If the key differs from `fxLastTRef` (the last APPLIED key) and no request is already in flight, call `requestFxOverlay` at `FX_SCALE` (0.5x) resolution.
7. In the response handler: clear `fxInflightRef` first (always, success or failure, so a rejected request can't wedge the loop). Drop the response outright if `isStaleFxResponse(requestedKey, fxWantRef.current)` - the desired key moved on while the request was outstanding, so applying it would blit an outdated spotlight/click frame; leaving it unlatched means the very next tick reissues a request for whatever is actually wanted now. Otherwise, only THEN latch `fxLastTRef` to the requested key - never before the request started - so a failed/rejected request or one that resolves `null` (nothing active) does not permanently mark that key "done" and block a retry; a non-null `url` decodes into `fxOverlayImgRef`, a `null` one explicitly clears it (`fxOverlayImgRef.current = null`) so a stale image never lingers after the spotlight/clicks end.

### Notes

- **Why the throttle/downscale exist:** the FX overlay is a full Tauri IPC round-trip (shader render, a per-pixel alpha-reconstruct pass, PNG encode, base64, a JS `Image` decode). The spotlight tracks the cursor, which moves on nearly every frame during playback, so a naive "request whenever the projected position changes" fires that round-trip ~60x/sec - this was the actual cause of a reported preview stutter. `FX_BUCKET_MS` caps the request cadence; `FX_SCALE` cuts the backend's per-pixel/PNG/decode cost by rendering at half linear resolution (the effects are soft gradients/ripples, invisible when the canvas upscales them on blit). Only one FX request is ever in flight at a time (the `fxInflightRef` gate), so when the round-trip takes longer than `FX_BUCKET_MS`, the effective update rate is bounded by that round-trip time, not the bucket - by design (this backpressure is what stops requests from piling up), not a bug.
- The FX-overlay coordinate mapping (`mapFn`, and the `(cx0,cy0,cw,ch)` crop it projects through) mirrors `drawPreview`'s whole-frame zoom crop exactly, just computed at `FX_SCALE` resolution - so spotlight/click positions land exactly where the zoomed base frame puts them. `cw`/`ch` are **rounded**, matching the export's `coordmap::crop`; leaving them fractional made the crop a sub-pixel different size, so every projected click and spotlight centre drifted slightly against the export.
- The `camRect` handed to `requestFxOverlay` (the spotlight's "don't dim the webcam" hole) is gated at `camAlpha > 0.05`, matching the export's `has_hole` test in `fx_state_at` - deliberately **not** the `0.004` threshold `layoutAt` uses to decide whether to *draw* the camera panel. A cross-fading panel passes through the gap between those two thresholds, where the preview would otherwise cut a hole the export never cuts. `camAlpha` is optional on `PreviewLayout` and treated as `1` when absent, since the static (non-segmented) layout carries no alpha and a cam rect there is fully shown.
- The FX math only ever reads the layout's `screen` rect (`lay.screen`), never `cam` - so the `camera_moves` override (which only ever touches `cam`) has no effect on spotlight/click/cursor positioning; it only changes where `drawPreview` draws the webcam PiP.
- `lastReportRef`, `fxOverlayImgRef`, `fxInflightRef`, `fxLastTRef`, `fxWantRef` are private to this hook (created via `useRef` inside it), since nothing outside the loop needs them. The cache-key/staleness helpers themselves (`timeBucket`, `fxCacheKey`, `isStaleFxResponse`) live in the sibling `fxCacheKey.ts` so they're unit-testable without a canvas/rAF loop - see `fxCacheKey.md`.
