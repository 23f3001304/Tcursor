# src/editor/stage/ZoomReticle.tsx

The on-stage aim reticle for a **Region**-target zoom: a crosshair-ring drawn over the preview at the point that zoom is aimed at. Purely presentational - it owns no state, no mapping math and no edit ops; `Stage` computes its position (`mapZoomTargetToCanvasPoint`) and owns the drag that moves it.

## ZoomReticle

```tsx
export function ZoomReticle({ x, y, aiming, dragging, onPointerDown }: {
  x: number; y: number; aiming: boolean; dragging: boolean;
  onPointerDown?: (e: React.PointerEvent) => void;
}): JSX.Element
```

### Props

- `x`, `y` - 0..1 fractions of `.e-stage` (which is the canvas' own displayed rect), applied as CSS `left`/`top` percentages. Values outside `0..1` are legal and simply clip against the stage's `overflow: hidden` - that is what an aim point currently cropped out of frame looks like.
- `aiming: boolean` - aim mode is active. Adds `.aim`, which is the **only** thing that turns `pointer-events` back on; without it the reticle is inert and the plain click-to-add-a-zoom gesture passes straight through it.
- `dragging: boolean` - the pointer currently owns the reticle. Adds `.drag` (grabbing cursor) **and** switches the position transition to `{ duration: 0 }`.
- `onPointerDown` - begins the aim drag. `Stage` passes it only in aim mode; `undefined` otherwise, so there is nothing to grab.

### Motion

Position animates with a Motion spring (`stiffness: 420, damping: 34, mass: 0.6`) so a click across the stage *settles* onto the new aim rather than teleporting - but the transition drops to zero duration while `dragging`, because a spring under the finger reads as input lag, not polish. `initial={false}` keeps the first paint from animating in from the top-left.

### Styling

`.e-zreticle` (in `editor.css`): a 30px ring in `--e-zoom` (the zoom lane's accent, so the reticle is colour-matched to the pill it belongs to) plus an inner `<i>` whose two pseudo-elements draw the crosshair arms overhanging the ring. `.aim` adds a focus halo; `.drag` swaps the cursor. Tokens only - the rgba fills are the literal `--e-zoom` colour at low alpha, matching how `.e-camdrag` handles `--e-cam`.
