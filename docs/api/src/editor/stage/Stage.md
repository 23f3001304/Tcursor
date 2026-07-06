# src/editor/stage/Stage.tsx

The preview stage: a vertical frame-tool strip (`StageToolbar`) on the left and a live composited preview on the right. The screen (a low-res proxy) and webcam play in hidden native `<video>` elements, a hidden `<audio>` plays the mixed preview audio, and `useCompositeLoop` runs the one `requestAnimationFrame` loop that composites each frame onto a 2D canvas via `drawPreview` (background + rounded zoomed screen + cursor + webcam PiP, matching the export's whole-frame zoom crop) plus the backend FX overlay (spotlight + click ripples) blitted on top. Native decode + `drawImage` holds 60fps. Clicking the canvas adds a zoom focused on the clicked point. This file stays thin - playback control (`useMediaPlayback`), live-value mirroring (`useSyncRefs`), sprite decoding (`useCursorSprites`), and the compositing loop itself (`useCompositeLoop`) are all extracted into sibling hooks.

## Stage

```tsx
export function Stage({ src, webcamSrc, track, layout, clicks, bgUrl, cursorSprites, cursorKinds, cursor, effects, clickfx, audioSrc, muted, timeMs, playing, onTime, onDuration, onZoomAt }): JSX.Element
```

Composites and shows the current preview frame, driven by the native `<video>`'s clock while playing and by `timeMs` while paused.

### Props

- `src: string` - the screen video URL (asset-protocol). The editor shows the raw capture first, then hot-swaps to the transcoded proxy; the swap is handled here without resetting the playhead.
- `webcamSrc: string` - the webcam video URL, drawn as the PiP panel (or `""`).
- `track: CamSample[]` - the exact camera curve; `camAt` interpolates it at the current time to drive the zoom + cursor position.
- `layout: PreviewLayout | null` - the export framing (screen rect + radius + webcam rect) as fractions of the canvas; `null` falls back to an inset.
- `clicks: ClickSample[]` - the mouse-down track, drawn as expanding ripples mapped through the zoom crop.
- `bgUrl: string` - the export background as a data URL, decoded once into an `<img>` the canvas draws under the screen.
- `cursorSprites: CursorSpriteDto[]` / `cursorKinds: CursorKindSample[]` / `cursor: CursorSettings` - the Capitaine sprite pack, the cursor-shape track, and the recording's cursor settings, so the preview draws the real export cursor (Enhanced style only).
- `effects: EffectRegion[]` - editor-added effect regions; the Spotlight ones light the spotlight overlay (recorded hotkey holds are seeded into `effects` as regions too, so no separate holds prop is needed here).
- `clickfx: ClickFxSettings` - the recording's click-FX + spotlight look (dim/radius/feather/mode/tint + the global toggle). Passed straight through to the backend FX-overlay request.
- `audioSrc: string` / `muted: boolean` - the mixed preview-audio URL played by a hidden `<audio>`, and the mute toggle.
- `timeMs: number` - the playhead; while paused it seeks the videos, while playing the video's own clock drives the frame.
- `playing: boolean` - play/pause; starts/stops the `<video>`/`<audio>` elements.
- `onTime: (ms: number) => void` - reports the video's current time while playing (the editor's playhead source of truth), throttled to ~16fps since it re-renders the whole editor tree; the canvas itself stays 60fps.
- `onDuration: (ms: number) => void` - reports the `<video>`'s true duration once metadata loads (the encoded length can differ from the seeded timeline span).
- `onZoomAt: (x: number, y: number) => void` - called with a 0..1 screen-content point when the canvas is clicked, so the editor adds a zoom focused there.

### Behavior

**Compositing loop.** `useCompositeLoop` (a sibling hook) runs the one `requestAnimationFrame` loop: reads the time (the screen `<video>`'s `currentTime` while playing, else `timeMs` via a ref), keeps the webcam and audio roughly synced to the screen video, builds the cursor input, calls `drawPreview`, and blits the backend FX overlay (spotlight + click ripples) on top - throttled and rendered at reduced resolution (see that file's notes on why). Live props are mirrored into refs by `useSyncRefs` so the single long-lived loop always sees current values without re-subscribing.

**Idle skip (perf).** When paused, the loop composites only when a draw-affecting input changed - a `dirtyRef` set by an effect on the draw inputs (`timeMs`, `track`, `layout`, `clicks`, `effects`, `cursor`, `clickfx`, `cursorKinds`, `playing`) and by the screen AND webcam videos' `loadeddata`/`seeked` (the webcam is composited every frame too, so a paused webcam seek must repaint) and the background/sprite `onload`. Otherwise it skips `drawPreview` entirely, so an open-but-idle editor doesn't burn 60fps redrawing the same static frame (this was the "editor lags at all times" bug).

**Throttled clock (perf).** While playing, `onTime` - which sets `timeMs` and re-renders the whole editor tree (Stage + Transport + the heavy Timeline) - fires at ~16fps (a 60ms gate), not every frame. The canvas preview still updates at 60fps because it reads the video's `currentTime` directly, not this throttled state; only the playhead/time-label lag by up to a frame (a spring smooths it). The `drawPreview` call is wrapped so a transient not-yet-decodable frame can't throw out of the loop and permanently freeze the preview (in dev it logs the error rather than swallowing it silently). Because the throttle can gate out the *final* report as `currentTime` plateaus at the end, the screen `<video>`'s `onEnded` reports the exact rounded duration, so playback still auto-stops at the end (`onTime`'s `ms >= dur` check).

**Paused seek.** A separate effect seeks the screen/webcam/audio elements to `timeMs` only when paused (guarded on `playRef.current`, not the captured `playing`, so a stale closure can't seek the video backward mid-play - the bug that made the playhead "loop" from the middle).

**Raw→proxy swap.** `onLoadedMetadata` reports the duration, restores `currentTime` from the ref, and resumes playback if it was playing - so swapping `src` from the raw capture to the proxy doesn't snap the playhead to 0.

**Click-to-zoom (`onCanvasClick`).** Delegates to `mapCanvasClickToZoomTarget` (`zoomTargetMapper.ts`), which un-projects the click through the current whole-frame zoom crop back to a 0..1 screen-content fraction, then calls `onZoomAt`. Clicks outside the screen rect (on the background or PiP) are ignored (`null`).

**Frame-tool strip.** `<StageToolbar />` - four icon buttons (aspect ratio, cursor, captions, 3D camera), decorative stubs for now.

### Notes

- The canvas is a fixed 1280x720 backing store; CSS scales it to fit. `onCanvasClick`/`mapCanvasClickToZoomTarget` divide by the displayed rect to recover backing-store pixels.
- The spotlight/click-FX overlay is rendered by the backend at pixel-perfect parity with the export (not approximated in Canvas2D) - see `previewCanvas.md`, `fxOverlay.md`, `useCompositeLoop.md`.
