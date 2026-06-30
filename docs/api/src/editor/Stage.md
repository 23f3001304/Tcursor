# src/editor/Stage.tsx

The preview stage: a vertical frame-tool strip on the left and a live composited preview on the right. The screen (a low-res proxy) and webcam play in hidden native `<video>` elements, a hidden `<audio>` plays the mixed preview audio, and one `requestAnimationFrame` loop composites each frame onto a 2D canvas via `drawPreview` (background + rounded zoomed screen + cursor + click ripples + spotlight + webcam PiP). Native decode + `drawImage` holds 60fps. Clicking the canvas adds a zoom focused on the clicked point.

## Stage

```tsx
export function Stage({ src, webcamSrc, track, layout, clicks, bgUrl, cursorSprites, cursorKinds, cursor, effects, spotlightHolds, clickfx, audioSrc, muted, timeMs, playing, onTime, onDuration, onZoomAt }): JSX.Element
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
- `effects: EffectRegion[]` - editor-added effect regions; the Spotlight ones light the spotlight overlay.
- `spotlightHolds: HoldSpan[]` - the recorded hotkey-hold spotlight intervals (output time), unioned with `effects` so a spotlight held while recording also shows.
- `clickfx: ClickFxSettings` - the recording's click-FX + spotlight look (dim/radius/feather + the global toggle). When `clickfx.enabled` is false the spotlight input is `null` (no FX).
- `audioSrc: string` / `muted: boolean` - the mixed preview-audio URL played by a hidden `<audio>`, and the mute toggle.
- `timeMs: number` - the playhead; while paused it seeks the videos, while playing the video's own clock drives the frame.
- `playing: boolean` - play/pause; starts/stops the `<video>`/`<audio>` elements.
- `onTime: (ms: number) => void` - reports the video's current time each frame while playing (the editor's playhead source of truth).
- `onDuration: (ms: number) => void` - reports the `<video>`'s true duration once metadata loads (the encoded length can differ from the seeded timeline span).
- `onZoomAt: (x: number, y: number) => void` - called with a 0..1 screen-content point when the canvas is clicked, so the editor adds a zoom focused there.

### Behavior

**Compositing loop.** One `requestAnimationFrame` loop reads the time (the screen `<video>`'s `currentTime` while playing, else `timeMs` via a ref), keeps the webcam and audio roughly synced to the screen video, builds the cursor + spotlight inputs from refs, and calls `drawPreview`. Live props are mirrored into refs so the single long-lived loop always sees current values without re-subscribing.

**Paused seek.** A separate effect seeks the screen/webcam/audio elements to `timeMs` only when paused (guarded on `playRef.current`, not the captured `playing`, so a stale closure can't seek the video backward mid-play - the bug that made the playhead "loop" from the middle).

**Raw→proxy swap.** `onLoadedMetadata` reports the duration, restores `currentTime` from the ref, and resumes playback if it was playing - so swapping `src` from the raw capture to the proxy doesn't snap the playhead to 0.

**Click-to-zoom (`onCanvasClick`).** Inverse-maps the click pixel through the current zoom crop (`sx,sy,sw,sh` derived exactly as `previewCanvas.ts` derives them) and the screen rect to a 0..1 screen-content fraction, then calls `onZoomAt`. Clicks outside the screen rect (on the background or PiP) are ignored.

**Frame-tool strip (`e-ftool`).** Four icon buttons (aspect ratio, cursor, captions, 3D camera) - decorative stubs for now.

### Notes

- The canvas is a fixed 1280x720 backing store; CSS scales it to fit. `onCanvasClick` divides by the displayed rect to recover backing-store pixels.
- The spotlight overlay approximates the export's Classic mode (a colorless darken); other spotlight modes are not yet previewed.
