# src/editor/stage/CamDragHandle.tsx

The Move-mode webcam drag handle: the `.e-camdrag` box that sits exactly over the PiP rect while "Move in preview" is on, so the webcam can be dragged directly in the preview. Split out of `Stage.tsx` (which was at the 200-line budget) when the zoom aim reticle landed - the logic is unchanged from the version that lived there.

## CamDragHandle

```tsx
export function CamDragHandle({ layout, layoutPresets, layoutSegs, cameraMoves, timeMs, playing,
  canvasW, canvasH, canvasRef, camDraftRef, dirtyRef }): JSX.Element | null
```

Returns `null` when the layout active at `timeMs` resolves no camera panel (e.g. `screen_only`) - there is nothing to drag.

### Props

- `layout` / `layoutPresets` / `layoutSegs` / `cameraMoves` / `timeMs` / `canvasW` / `canvasH` - everything needed to recompute the PiP rect for this frame; see Behavior. `cameraMoves` doubles as one of the two draft-clear triggers - see Notes.
- `playing: boolean` - `Stage`'s own `playing` prop, passed straight through (review round 1, Important 3/M6 - replaces the earlier `draggingRef` prop). Needed to tell an ordinary playback tick apart from a real seek (`isNaturalPlaybackTick`, `./playbackTick.ts`) for the `[timeMs]` effect below; mirrored into `playingRef` (reassigned every render) so the effect - keyed on `[timeMs]` only, matching the pre-existing pattern - always reads the CURRENT value instead of whatever it was when the effect last re-ran.
- `canvasRef` - the preview `<canvas>`, whose bounding rect `mapPointerToCamFraction` divides by.
- `camDraftRef` - the shared UNSAVED pose (lifted to `Editor`, read every frame by `useCompositeLoop`, saved by `CameraPanel`'s Update/Add button, which also CLEARS it once used - see `CameraPanel.md`). This component writes it; `Stage` clears it on a real `timeMs` discontinuity (see `Stage.md`).
- `dirtyRef` - `Stage`'s repaint flag, set on every drag step so a paused preview recomposites.

### Behavior

**Pose derivation.** `panel` is computed the same way `frameCamLayout` computes `frameLayout.cam`: `layoutAt` -> `liveCamPose` -> `camMoveAt` -> `overrideCamPanel` (or the layout's own panel when nothing overrides it), so a Wide (16:9) PiP's handle is wide too, and its rounding follows the OVERRIDDEN panel's radius - a keyframed shape, or a morph between two, is what the handle hugs. The result is in fractions of the canvas, and `.e-stage`'s box IS the canvas' displayed rect, so they convert straight to CSS percentages. `handleRadiusPct` mirrors the webcam's radius/width ratio so the handle hugs a round PiP instead of boxing it.

**Drag.** Pointer-down stops propagation (so the drag is not also a click-to-zoom) and captures the current size (`camDraftRef.current ?? sampledPose ?? livePose`, else `0.25`). Movement past a 4px threshold (`pastDragThreshold`, `../hooks/dragThreshold.ts`) - so a plain click moves nothing - maps the pointer to a plain frame fraction via `mapPointerToCamFraction` (no zoom un-projection, unlike the canvas click: the PiP is the fixed top layer) and writes it into `camDraftRef` plus local `dragPose` state. `attachPointerGesture` (`./pointerGesture.ts`) keeps the drag alive past the small handle and wires `pointercancel` alongside `pointerup`; its returned `detach` is stashed in a ref and called from a `useEffect` unmount cleanup, so a Move-mode toggle-off mid-drag can't leave `pointermove`/`pointerup`/`pointercancel` bound to `window` forever (bug-sweep-2 Task 8, L4).

**No commit on release.** The gesture's `onEnd` only detaches listeners. The dragged pose stays drawn and shown until either `CameraPanel`'s Update/Add button reads `camDraftRef` and writes a real `camera_moves` keyframe (clearing the draft itself as part of that), or the playhead makes a real seek.

### Notes

- The draft reset is split across two files on purpose: this component drops its own `dragPose` on a `timeMs` discontinuity, while `Stage` clears the shared `camDraftRef` the same way. `Stage` must keep that half, because the ref has to be cleared even when Move mode is off and this component is unmounted.
- **`[timeMs]` effect uses `isNaturalPlaybackTick`, not a `draggingRef` (bug-sweep-2 Task 8, M6; redesigned in review round 1, Important 3).** The FIRST fix here flipped a shared `draggingRef` boolean true only while an ACTIVE, past-threshold drag was in progress, and gated the reset on it - but `draggingRef` flipped back to `false` the INSTANT the pointer was released, so the very next natural playback tick (within ~60ms, since Move mode + Play can run together) still nulled the draft right after letting go: "Add keyframe at playhead" could still write frame-centre if pressed even slightly after the drag ended, not just during it. The current design instead asks "was this timeMs change an ordinary playback tick, or a real seek" (`isNaturalPlaybackTick(lastMsRef.current, timeMs, playingRef.current)`) - the draft now survives EVERY ordinary tick, whether a drag is in progress, just released, or long since released, and clears only on an actual discontinuity (a scrub/skip/loop) or the OTHER trigger below.
- **`[cameraMoves]` effect - the "explicit action" clear trigger, gated on CONTENT not REFERENCE (bug-sweep-2 Task 8, M6; corrected in review round 2, Important).** `CameraPanel`'s "Add keyframe" button consumes the draft by writing a real `camera_moves` entry (`addKeyframeHere` - see `CameraPanel.md`; the "Webcam size" slider, `setKfSize`, deliberately does NOT consume it - see that file's own note), so this effect clears the LOCAL `dragPose` mirror the same moment `CameraPanel` clears the ref copy (`camDraftRef.current = null`). The effect is still keyed on `[cameraMoves]` (the prop reference), but INSIDE it now compares `cameraMovesKey(cameraMoves)` (`./cameraMoves.ts`) against the key stored from the last time it ran, and only clears when that key actually differs. **The reference alone is NOT a valid signal**: `applyEditOp` round-trips the whole `EditDoc` through IPC, so `doc.camera_moves` gets a brand-new array reference on EVERY edit routed through it - add a zoom, delete, trim, an AI step - not just camera-move ones. The original (round 1) version keyed directly on the reference and cleared on ALL of those; repro: Move-mode drag the PiP (draft uncommitted), press Z to add a zoom - the mirror nulled, snapping the drag-handle overlay to `sampledPose`, while the composited canvas (still reading the un-cleared `camDraftRef.current`) kept drawing the actual drag position - two on-screen elements visibly disagreeing. Without this half at all, the HANDLE's own CSS position (driven by `dragPose`, not `camDraftRef` directly) would keep showing the stale dragged pose after a real commit, even though the ref itself and the rAF-loop-drawn PiP had already moved on.
- Mutually exclusive with the zoom aim reticle - see `Editor.md`'s aim-mode note. Both would otherwise claim the same pointer on the same canvas.
