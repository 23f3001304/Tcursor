# src/editor/stage/CamDragHandle.tsx

The Move-mode webcam drag handle: the `.e-camdrag` box that sits exactly over the PiP rect while "Move in preview" is on, so the webcam can be dragged directly in the preview. Split out of `Stage.tsx` (which was at the 200-line budget) when the zoom aim reticle landed - the logic is unchanged from the version that lived there.

## CamDragHandle

```tsx
export function CamDragHandle({ layout, layoutPresets, layoutSegs, cameraMoves, timeMs,
  canvasW, canvasH, canvasRef, camDraftRef, dirtyRef }): JSX.Element | null
```

Returns `null` when the layout active at `timeMs` resolves no camera panel (e.g. `screen_only`) - there is nothing to drag.

### Props

- `layout` / `layoutPresets` / `layoutSegs` / `cameraMoves` / `timeMs` / `canvasW` / `canvasH` - everything needed to recompute the PiP rect for this frame; see Behavior.
- `canvasRef` - the preview `<canvas>`, whose bounding rect `mapPointerToCamFraction` divides by.
- `camDraftRef` - the shared UNSAVED pose (lifted to `Editor`, read every frame by `useCompositeLoop`, saved by `CameraPanel`'s Update/Add button). This component writes it; `Stage` clears it when the playhead moves.
- `dirtyRef` - `Stage`'s repaint flag, set on every drag step so a paused preview recomposites.

### Behavior

**Pose derivation.** `pipRect` is computed the same way `useCompositeLoop` computes `frameLayout.cam`: `layoutAt` -> `camMoveAt` (or the static rect) -> `rectFromCenter` at the static panel's own `camAspect`, so a Wide (16:9) PiP's handle is wide too. The result is in fractions of the canvas, and `.e-stage`'s box IS the canvas' displayed rect, so they convert straight to CSS percentages. `handleRadiusPct` mirrors the webcam's radius/width ratio so the handle hugs a round PiP instead of boxing it.

**Drag.** Pointer-down stops propagation (so the drag is not also a click-to-zoom) and captures the current size (`camDraftRef.current ?? sampledPose ?? livePose`, else `0.25`). Movement past a 4px threshold - so a plain click moves nothing - maps the pointer to a plain frame fraction via `mapPointerToCamFraction` (no zoom un-projection, unlike the canvas click: the PiP is the fixed top layer) and writes it into `camDraftRef` plus local `dragPose` state. Window `pointermove`/`pointerup` listeners keep the drag alive past the small handle.

**No commit on release.** `up` only detaches listeners. The dragged pose stays drawn and shown until either `CameraPanel`'s Update/Add button reads `camDraftRef` and writes a real `camera_moves` keyframe, or the playhead moves.

### Notes

- The draft reset is split across two files on purpose: this component drops its own `dragPose` on a `timeMs` change, while `Stage` clears the shared `camDraftRef`. `Stage` must keep that half, because the ref has to be cleared even when Move mode is off and this component is unmounted.
- Mutually exclusive with the zoom aim reticle - see `Editor.md`'s aim-mode note. Both would otherwise claim the same pointer on the same canvas.
