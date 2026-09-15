# src/editor/stage/useStagePointer.ts

Everything the stage canvas does with a pointer, in one hook: the inverse mapping from a screen position to a zoom target, what a click on the canvas means, the reticle drag, and where the reticle sits right now.

**Why it exists.** `Stage.tsx` stood at exactly its 200-line cap, and the AI review sheet's one `outline` prop had nowhere to land (M4 plan, A12). The move is a pure lift: the same guards, the same call order, the same `useReticleDrag` wiring, no behaviour change. The stage suite passed unchanged across it.

## useStagePointer

```ts
export function useStagePointer({
  canvasRef, screenRef, layoutRef, trackRef, mapRef, timeRef,
  arranging, aimMode, aimPoint, playing, canvasW, canvasH, layout, track, tOut, onZoomAt, onAimAt,
}): {
  targetUnderPointer: (clientX: number, clientY: number) => [number, number] | null;
  onCanvasClick: (e: React.MouseEvent<HTMLCanvasElement>) => void;
  reticle: [number, number] | null;
  aimDrag: boolean;
  onReticleDown: (e: React.PointerEvent) => void;
}
```

### Props

- **The refs** (`layoutRef` / `trackRef` / `mapRef` / `timeRef`) are the ones `Stage` already keeps for its rAF loop, and are read at CALL time: a click maps through the frame that is actually on screen, not the one React last rendered against.
- **The plain values** (`layout` / `track` / `tOut` / `canvasW` / `canvasH`) drive the reticle, which is render state and must move with the component.

### Behavior

**`targetUnderPointer`** un-projects a screen position through the current whole-frame crop and the screen panel's rect into a 0..1 screen-content fraction - the zoom target's own basis. `null` when the canvas or the proxy video is not ready.

**`onCanvasClick`** adds a zoom at that point, EXCEPT in aim mode, where it re-aims the selected Region zoom instead, and except while arranging, where the click belongs to the panel frames and must not also drop a zoom behind them. The three stage modes claim the same pointer, and this is where two of them are resolved.

**`reticle`** is where the stored aim point lands on the canvas right now (the crop moves it as the camera ramps), hidden during playback and while arranging - it is an editing affordance, not a playback overlay. The live drag override (`useReticleDrag`) wins over the stored point while a drag is in flight.

### Used by

- `src/editor/stage/Stage.tsx` - one call; `targetUnderPointer` is returned for completeness and is consumed internally by the reticle drag.
