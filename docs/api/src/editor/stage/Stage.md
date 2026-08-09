# src/editor/stage/Stage.tsx

The preview stage: a vertical frame-tool strip (`StageToolbar`) on the left and a live composited preview on the right. The screen (a low-res proxy) and webcam play in hidden native `<video>` elements, a hidden `<audio>` plays the mixed preview audio, and `useCompositeLoop` runs the one `requestAnimationFrame` loop that composites each frame onto a 2D canvas via `drawPreview` (background + rounded zoomed screen + cursor + webcam PiP, matching the export's whole-frame zoom crop) plus the backend FX overlay (spotlight + click ripples) blitted on top. Native decode + `drawImage` holds 60fps. Clicking the canvas adds a zoom focused on the clicked point. This file stays thin - playback control (`useMediaPlayback`), live-value mirroring (`useSyncRefs`), sprite decoding (`useCursorSprites`), the compositing loop itself (`useCompositeLoop`), and the hidden media elements' markup (`StageMedia`) are all extracted into sibling hooks/components.

## Stage

```tsx
export function Stage({ src, webcamSrc, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, bgUrl, cursorSprites, cursorKinds, osCursorInVideo, cursor, effects, clickfx, audioSrc, muted, volume, timeMs, playing, moveMode, aimPoint, aimMode, camDraftRef, onTime, onDuration, onZoomAt, onAimAt, onRetryMedia }): JSX.Element
```

Composites and shows the current preview frame, driven by the native `<video>`'s clock while playing and by `timeMs` while paused.

### Props

- `src: string` - the screen video URL (asset-protocol). The editor shows the raw capture first, then hot-swaps to the transcoded proxy; the swap is handled here without resetting the playhead.
- `webcamSrc: string` - the webcam video URL, drawn as the PiP panel (or `""`).
- `osCursorInVideo: boolean` - whether the recording's video already has the OS cursor baked into its pixels (from the `os_cursor_in_video` IPC, i.e. the record-time settings snapshot). Drives the **plain-OS fallback** below. Defaults to the safe `true` (draw nothing) while the fetch is in flight.
- `track: CamSample[]` - the exact camera curve; `camAt` interpolates it at the current time to drive the zoom + cursor position.
- `layout: PreviewLayout | null` - the export framing (screen rect + radius + webcam rect) as fractions of the canvas; `null` falls back to an inset.
- `layoutPresets: LayoutPresets | null` / `layoutSegs: LayoutSeg[]` - all 5 layout presets' panel rects and `doc.layout`'s segments, so the preview cross-fades across layout-segment boundaries itself (`layoutAt`) instead of only showing the single static `layout` above.
- `cameraMoves: CameraMove[]` - `doc.camera_moves`; inside the span they own (`[first - KF_BLEND_MS, last + KF_BLEND_MS]`, Task 27), `camMoveAt` overrides the resolved layout's webcam rect each frame (position + size only - radius/alpha stay as resolved), so the preview PiP follows the same track the export does. Empty, or anywhere outside that span, is a no-op and the layout segments own the panel. `camMoveAt`'s `live` argument (`livePose`, derived from `baseLayout.cam`) is the layout-resolved pose the track eases to and from at the span's edges, and is also what a Move-mode drag seeds its starting size from when the sample is `null` - mirrors the export's `step_camera`/`static_cam_pose`.
- `zooms: Zoom[]` / `zoomSettings: ZoomSettings` - `doc.zooms` and `doc.settings.zoom`; mirrored into refs and handed to `useCompositeLoop`, which uses them to shrink/reposition the webcam PiP during an active zoom (`camera_shrink`/`camera_shrink_min` etc.), mirroring the export's `step_camera`. `Stage` itself only forwards these - it doesn't read them directly.
- `clicks: ClickSample[]` - the mouse-down track, drawn as expanding ripples mapped through the zoom crop.
- `bgUrl: string` - the export background as a data URL, decoded once into an `<img>` the canvas draws under the screen.
- `cursorSprites: CursorSpriteDto[]` / `cursorKinds: CursorKindSample[]` / `cursor: CursorSettings` - the Capitaine sprite pack, the cursor-shape track, and the recording's cursor settings, so the preview draws the real export cursor (Enhanced style only).
- `effects: EffectRegion[]` - editor-added effect regions; the Spotlight ones light the spotlight overlay (recorded hotkey holds are seeded into `effects` as regions too, so no separate holds prop is needed here).
- `clickfx: ClickFxSettings` - the recording's click-FX + spotlight look (dim/radius/feather/mode/tint + the global toggle). Passed straight through to the backend FX-overlay request.
- `audioSrc: string` / `muted: boolean` - the mixed preview-audio URL played by a hidden `<audio>`, and the mute toggle.
- `volume: number` - preview-audio gain as a `0..1` fraction - `Editor`'s `volume` state (shown `0..100` by `Transport`'s slider) divided by 100. Applied to the hidden `<audio>` element by `useMediaPlayback`, independently of `muted`.
- `timeMs: number` - the playhead; while paused it seeks the videos, while playing the video's own clock drives the frame.
- `playing: boolean` - play/pause; starts/stops the `<video>`/`<audio>` elements (via `useMediaPlayback`).
- `moveMode: boolean` - "Move in preview" (`CameraPanel`'s toggle). When on, renders `CamDragHandle` (the `.e-camdrag` box over the current PiP rect) so the user can drag the webcam directly in the preview.
- `aimPoint: [number, number] | null` - the selected zoom's stored **Region** target (0..1 screen-content), from `zoomTargetPoint`. Non-null is what makes the aim reticle exist at all: a cursor-following zoom, or no selection, is `null` and draws nothing.
- `aimMode: boolean` - aim mode is active (`Editor` only reports `true` when a Region zoom is selected AND camera Move mode is off). It flips the canvas click's meaning from add-a-zoom to re-aim, swaps the cursor to a crosshair, and is the only condition under which the reticle accepts a pointer.
- `camDraftRef: RefObject<CamPose | null>` - the shared, UNSAVED Move-mode camera pose: lifted to `Editor` and shared with `CameraPanel`'s Update/Add button. The composite loop draws the PiP from this ref whenever it's non-null; dragging the handle here writes into it live but does **not** commit a `camera_moves` keyframe - only the panel's save button does that, and moving the playhead clears it back to `null`. Replaces the old `applyOp` prop: `Stage` no longer calls the edit-op applier itself, so it doesn't need `EditOp`/`EditDoc` in scope at all.
- `onTime: (ms: number) => void` - reports the video's current time while playing (the editor's playhead source of truth), throttled to ~16fps since it re-renders the whole editor tree; the canvas itself stays 60fps.
- `onDuration: (ms: number) => void` - reports the `<video>`'s true duration once metadata loads (the encoded length can differ from the seeded timeline span).
- `onZoomAt: (x: number, y: number) => void` - called with a 0..1 screen-content point when the canvas is clicked, so the editor adds a zoom focused there. **Not** called in aim mode.
- `onAimAt: (x: number, y: number) => void` - the aim-mode counterpart: re-aims the already-selected zoom (`update_zoom { target: { fixed } }`) instead of adding a new one. Called on a canvas click in aim mode and on every step of a reticle drag.
- `onRetryMedia: () => void` - `useEditorData`'s `retryMedia` (bumps its internal `reloadTick`, included in the proxy-source effect's deps - a genuinely fresh `ensureProxy` attempt on the `fetch` branch). Called from `StageMedia`'s error card's Retry button, via a handler defined here (`onRetry={() => { setErr(null); onRetryMedia(); screen.current?.load(); }}`) - see Behavior.

### Behavior

**Compositing loop.** `useCompositeLoop` (a sibling hook) runs the one `requestAnimationFrame` loop: reads the time (the screen `<video>`'s `currentTime` while playing, else `timeMs` via a ref), keeps the webcam and audio roughly synced to the screen video, shrinks/repositions the webcam PiP for any active zoom via the `zooms`/`zoomSettings` refs (mirrors the export's `step_camera`), builds the cursor input, calls `drawPreview`, and blits the backend FX overlay (spotlight + click ripples) on top - throttled and rendered at reduced resolution (see that file's notes on why). Live props are mirrored into refs by `useSyncRefs` so the single long-lived loop always sees current values without re-subscribing.

**Idle skip (perf).** When paused, the loop composites only when a draw-affecting input changed - a `dirtyRef` set by an effect on the draw inputs (`timeMs`, `playing`, `track`, `layout`, `layoutPresets`, `layoutSegs`, `cameraMoves`, `zooms`, `zoomSettings`, `clicks`, `effects`, `cursor`, `clickfx`, `cursorKinds`) and by the screen AND webcam videos' `loadeddata`/`seeked` (the webcam is composited every frame too, so a paused webcam seek must repaint) and the background/sprite `onload`. Otherwise it skips `drawPreview` entirely, so an open-but-idle editor doesn't burn 60fps redrawing the same static frame (this was the "editor lags at all times" bug).

**Throttled clock (perf).** While playing, `onTime` - which sets `timeMs` and re-renders the whole editor tree (Stage + Transport + the heavy Timeline) - fires at ~16fps (a 60ms gate), not every frame. The canvas preview still updates at 60fps because it reads the video's `currentTime` directly, not this throttled state; only the playhead/time-label lag by up to a frame (a spring smooths it). The `drawPreview` call is wrapped so a transient not-yet-decodable frame can't throw out of the loop and permanently freeze the preview (in dev it logs the error rather than swallowing it silently). Because the throttle can gate out the *final* report as `currentTime` plateaus at the end, the screen `<video>`'s `onEnded` reports the exact rounded duration, so playback still auto-stops at the end (`onTime`'s `ms >= dur` check).

**Playback control (`useMediaPlayback`).** A sibling hook now owns starting/stopping the media on `playing` transitions - seeking to the current time *before* calling `.play()`, so a `Play` that also snaps the playhead (e.g. forward to the trim-in point, see `Transport`) reaches the actual `<video>`/`<audio>` elements in the same commit instead of resuming from a stale position - applying `volume`/`muted` to the hidden `<audio>` element, and re-seeking the screen/webcam/audio elements to `timeMs` whenever paused (guarded on `playRef.current`, not the captured `playing`, so a stale closure can't seek the video backward mid-play - the bug that made the playhead "loop" from the middle).

**Raw→proxy swap.** `onLoadedMetadata` reports the duration, restores `currentTime` from the ref, and resumes playback if it was playing - so swapping `src` from the raw capture to the proxy doesn't snap the playhead to 0.

**Click-to-zoom / click-to-aim (`onCanvasClick`).** One shared `targetUnderPointer` helper delegates to `mapCanvasClickToZoomTarget` (`zoomTargetMapper.ts`), which un-projects the pointer through the current whole-frame zoom crop back to a 0..1 screen-content fraction. The result then goes to `onAimAt` in aim mode, else `onZoomAt` - the two are mutually exclusive meanings for the same click. Clicks outside the screen rect (on the background or PiP) are ignored (`null`).

**Aim reticle (`reticle` + `onReticleDown`).** When `aimPoint` is set and playback is paused, `mapZoomTargetToCanvasPoint` projects that stored target back onto the canvas for THIS frame (the crop moves it as a zoom ramps) and `ZoomReticle` renders there. It is hidden during playback on purpose: it is an editing affordance, not a playback overlay, and a spring chasing a per-frame camera would read as lag. In aim mode the reticle is draggable - pointer-down stops propagation and attaches window `pointermove`/`pointerup`, and every move calls `onAimAt`. There is **no** new undo machinery: each move is an ordinary `update_zoom`, so `useEditHistory`'s `COALESCE_MS` window already folds a whole drag into one undo step.

**Move-mode drag.** Delegated to `CamDragHandle` (see `CamDragHandle.md`), rendered only while `moveMode` is on. `Stage` keeps just one half of the draft lifecycle: its `[timeMs]` effect clears the shared `camDraftRef`, which must happen even when Move mode is off and the handle is unmounted.

**Frame-tool strip.** `<StageToolbar />` - four icon buttons (aspect ratio, cursor, captions, 3D camera), decorative stubs for now.

**Media-error retry.** On Retry (`StageMedia`'s error card), `Stage` clears its own `err` (so the card disappears immediately rather than waiting for a reload to actually succeed), calls `onRetryMedia()` (may re-run `ensureProxy` on the backend), AND calls `screen.current?.load()` directly on the native `<video>` element. The `.load()` call is necessary because `srcUrl` is deterministic - `planProxySrc` resolves to the exact same filename on retry whenever the `immediate`/`known` branches apply, so merely re-running the effect and calling `setSrcUrl` with an unchanged string is a React no-op (`Object.is`-equal, no re-render, no DOM change) that would never actually re-attempt loading. `HTMLMediaElement.load()` is the platform API for "reload the resource at the current `src`", so it works regardless of which `planProxySrc` branch produced that `src`.

### Notes

- The canvas backing store follows `layout.canvas` (from `PreviewLayout`, resolved server-side via `Layout::resolve` from `EditDoc.aspect`) - `DEFAULT_CANVAS` (1280x720) is only the pre-load fallback. `.e-stage`'s `aspect-ratio` is set inline from the same `[canvasW, canvasH]` so the stage box, the canvas, and every fraction in this file share one basis regardless of the chosen aspect. CSS then scales that box to fit; `onCanvasClick`/`mapCanvasClickToZoomTarget` divide by the displayed rect to recover backing-store pixels.
- The spotlight/click-FX overlay is rendered by the backend at pixel-perfect parity with the export (not approximated in Canvas2D) - see `previewCanvas.md`, `fxOverlay.md`, `useCompositeLoop.md`.

### Plain-OS cursor fallback

`cursor.style === "system" && !osCursorInVideo` is the case the renderer calls plain-OS: the doc asks for the system cursor, but the video was recorded in `Enhanced` (or `Hidden`) and has none baked in. Rust re-creates it from the recorded path (`cursorset::draw`); Stage mirrors that decision for the canvas preview **without any new drawing code**, by feeding `useSyncRefs` an *effective* cursor instead of the raw prop:

- `style: "enhanced"` - flips on the existing `drawCursorSprite` gate, which is a plain style check.
- an empty `cursorKinds` array - `cursorAt` then always resolves to `"arrow"`, matching the renderer's always-Arrow rule.
- `click_bounce: false`, `motion_blur: 0` - the polish a real OS cursor doesn't have.

*Why nothing here handles the raw path:* preview cursor positions come from `camera_track`, which Rust computes with `CursorSettings::follow_alpha_at` - already un-smoothed in this mode. Only the sprite-side knobs need mirroring.

*Why the transform lives in Stage rather than in `useCompositeLoop` or `cursorPreview`:* both of those consume a `CursorSettings`/kinds pair that Stage already owns, so expressing the fallback as "effective settings" keeps the loop and the draw function completely unchanged.
