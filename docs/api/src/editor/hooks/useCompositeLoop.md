# src/editor/hooks/useCompositeLoop.ts

The Stage's single `requestAnimationFrame` loop, extracted from `Stage.tsx` so that file stays under the size limit. Reads the live time, keeps the webcam/audio roughly synced to the screen video, composites the base frame via `drawPreview`, blits the cached backend FX overlay on top, and throttles/downscales fresh FX-overlay requests.

## useCompositeLoop

```ts
export function useCompositeLoop({
  screenRef, webcamRef, audioRef, canvasRef,
  playRef, timeRef, onTimeRef,
  trackRef, layoutRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  spritesRef, trailRef, dirtyRef, bgImgRef,
}: { /* all live values as RefObjects, mirrored by Stage.tsx / useSyncRefs */ }): void
```

### Inputs

All inputs are `RefObject`s (not plain values) so the one long-lived `rAF` loop always reads current state without re-subscribing - `Stage.tsx` owns the refs (via `useSyncRefs` for the shared ones) and this hook only reads them each tick. See `Stage.tsx` for what each one holds; `dirtyRef` and `bgImgRef` are Stage's own refs (paused-frame dirty flag, decoded background image).

### Returns

`void` - side-effect only; starts the loop on mount, cancels it on unmount.

### Implementation

1. Each tick: if paused and nothing is `dirtyRef`, skip entirely (the idle/interaction-lag fix - an open-but-idle editor shouldn't burn 60fps redrawing a static frame).
2. Read the time (`sv.currentTime` while playing, else `timeRef`), throttle the `onTime` report to ~16fps, nudge webcam/audio back in sync if they've drifted.
3. Build the cursor input and call `drawPreview` (background + screen + webcam + cursor, no FX) into an offscreen buffer it owns (`offscreenRef`, created lazily, resized to the canvas size by `drawPreview` itself).
4. Blit the cached FX-overlay image (`fxOverlayImgRef`) on top, if one exists and has decoded.
5. Compute a cache key from the zoom-projected cursor position, spotlight params, click track, and click-FX params, bucketed to `FX_BUCKET_MS` (~25fps) - **not** every rAF tick. If the key changed and no request is already in flight, call `requestFxOverlay` at `FX_SCALE` (0.5x) resolution and swap in the decoded result (or clear it) when it resolves.

### Notes

- **Why the throttle/downscale exist:** the FX overlay is a full Tauri IPC round-trip (shader render, a per-pixel alpha-reconstruct pass, PNG encode, base64, a JS `Image` decode). The spotlight tracks the cursor, which moves on nearly every frame during playback, so a naive "request whenever the projected position changes" fires that round-trip ~60x/sec - this was the actual cause of a reported preview stutter. `FX_BUCKET_MS` caps the request cadence; `FX_SCALE` cuts the backend's per-pixel/PNG/decode cost by rendering at half linear resolution (the effects are soft gradients/ripples, invisible when the canvas upscales them on blit).
- The FX-overlay coordinate mapping (`mapFn`, and the `(cx0,cy0,cw,ch)` crop it projects through) mirrors `drawPreview`'s whole-frame zoom crop exactly, just computed at `FX_SCALE` resolution - so spotlight/click positions land exactly where the zoomed base frame puts them.
- `lastReportRef`, `fxOverlayImgRef`, `fxInflightRef`, `fxLastTRef` are private to this hook (created via `useRef` inside it), since nothing outside the loop needs them.
