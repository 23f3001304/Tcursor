# src/editor/stage/camDragMapper.ts

Maps a pointer position over the preview `<canvas>` to a plain 0..1 fraction of the displayed frame, for dragging the webcam PiP in Move mode. Deliberately simpler than `zoomTargetMapper.ts`: the webcam PiP is the FIXED TOP layer in OUTPUT space (composited after the screen's zoom/crop - see the Rust `Scene.camera`), so it never needs un-projecting through the current zoom the way a zoom-target click does.

## mapPointerToCamFraction

```ts
export function mapPointerToCamFraction({
  clientX, clientY, canvasElement,
}: {
  clientX: number; clientY: number; canvasElement: HTMLCanvasElement;
}): [number, number]
```

### Inputs

- `clientX`, `clientY` - the pointer's viewport coordinates (from a `PointerEvent`/`React.PointerEvent`).
- `canvasElement` - the preview `<canvas>`; its bounding rect converts client coords to backing-store pixels, its `width`/`height` are the projection's frame size (sized from `PreviewLayout.canvas`, following the doc's chosen aspect - see `Stage.tsx`).

### Returns

`[number, number]` - a 0..1 fraction of the canvas' displayed frame, always clamped into range (never `null` - unlike `mapCanvasClickToZoomTarget`, there's no "outside the panel" rejection, since the PiP can be dragged to any point in the frame).

### Implementation

1. Convert the pointer position to canvas backing-store pixels via `canvasElement.getBoundingClientRect()` - the same letterbox math `zoomTargetMapper.ts` uses (reused, not duplicated logic-for-logic, since both need "client px -> backing-store px").
2. Divide by the canvas' own `width`/`height` to get a plain fraction - no screen-panel rect, no zoom-crop un-projection (the part `zoomTargetMapper` needs and this deliberately drops).
3. Clamp both axes to `[0, 1]`.

### Notes

- **Why this can't reuse `mapCanvasClickToZoomTarget` outright:** that function's whole job is un-projecting through the *screen* layer's zoom crop, because a zoom target lives in screen-content space. The webcam PiP lives in output-frame space and is drawn after any zoom/crop is applied, so applying that same un-projection here would move the PiP to the wrong point whenever a zoom was active. Only the canvas-display-rect (letterbox) math is shared between the two.
- Used by `Stage.tsx`'s Move-mode drag handle (`onHandlePointerDown` and the drag-move listener) to compute the live `CamPose.x`/`.y` each pointer move.
