# src/editor/stage/mask/MaskOverlay.tsx

The selected mask's drag box on the stage: the rectangle, its eight grips, and the snapping guides.

## MaskOverlay

```tsx
export function MaskOverlay({ box, canvasW, canvasH, guideX, guideY, onHandleDown }: {
  box: MaskPx; canvasW: number; canvasH: number;
  guideX: number | null; guideY: number | null;
  onHandleDown: (e: React.PointerEvent, handle: MaskHandle) => void;
}): JSX.Element
```

Pure presentation: every number it draws is already decided by `useMaskDrag`, and it holds no state.

**Positioned in PERCENTAGES, not pixels.** `box` arrives in canvas pixels and each edge is divided by the canvas size before being written to `style`, so the overlay tracks the canvas element however the stage is letterboxed or scaled by `viewMode`. A pixel `left` would be correct only at one window size.

**Eight grips, one handler.** The `HANDLES` array is walked in clockwise order from `nw`, each `<i>` carrying its compass class for `stage.css` to place and to give the right resize cursor. The body itself takes `"move"`. Every grip calls the same `onHandleDown`, which is why `MaskHandle` is a string the geometry can test rather than nine separate callbacks. `useMaskDrag`'s `onHandleDown` calls `stopPropagation`, so a press on a grip does not also reach the box beneath it.

**The wrapper is pointer-transparent.** `.e-maskwrap` covers the stage with `pointer-events: none` and only the box and the grips turn it back on, so a click anywhere else on the stage still reaches the canvas and its other pointer modes.

### Motion

The guides fade with `AnimatePresence` and a 100ms `easeOut` tween - stateful appearance and disappearance, which is what the Motion rule reserves it for. The box and the grips are plain elements: they follow the pointer every frame and a spring on them would lag the drag.

`.e-aguide` is the arrange overlay's own guide class, reused verbatim rather than restyled, so snapping a mask and snapping a panel look like one idea rather than two features that each invented a line.

### Used by

- `src/editor/stage/Stage.tsx` - one sibling element after the `CamDragHandle` block, rendered while `mask.box` exists and neither arrange nor Move mode owns the pointer.
