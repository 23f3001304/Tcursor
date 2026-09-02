# src/editor/hooks/dragThreshold.ts

Pure "has this pointer actually dragged?" hit-test, extracted so it's unit-testable without a DOM and shared by every timeline/stage drag that must not commit a no-op edit just because the user clicked to select something (bug-sweep-2 Task 8, D-Medium M8; UX audit #5 - a stationary click on a spotlight pill silently nudged its Start `0s -> 0.03s`).

## DRAG_THRESHOLD_PX

```ts
export const DRAG_THRESHOLD_PX = 3
```

The default threshold (px). Close to `CamDragHandle`'s own pre-existing 4px guard without being tied to its exact number - callers that want a different threshold (e.g. `CamDragHandle` itself, which keeps its historical 4px) pass it explicitly to `pastDragThreshold`.

## pastDragThreshold

```ts
export function pastDragThreshold(dx: number, dy: number, px: number = DRAG_THRESHOLD_PX): boolean
```

### Inputs

- `dx` / `dy` - the pointer's total displacement (px) from wherever the drag began, NOT a per-move delta. Callers compute this from a `clientX`/`clientY` recorded at `pointerdown` vs. the current (or release) event.
- `px` - the threshold; defaults to `DRAG_THRESHOLD_PX`.

### Returns

`Math.hypot(dx, dy) >= px` - Euclidean distance, not per-axis, so a diagonal jiggle under the threshold is still ignored even when its X or Y component alone would exceed it.

### Used by

- `useRegionDrag.ts` - `up` only calls `onCommit` when the RELEASE position is past the default threshold from `beginDrag`'s start; a bare click still selects (via `beginDrag`'s unconditional `onSel`) but commits nothing.
- `timeline/TrimOverlay.tsx` - two independent checks (review round 1 minor, unified with `useRegionDrag`/`CameraLane`): `move` withholds any LIVE drag-value update until past the default threshold on the X axis alone (`pastDragThreshold(dx, 0)`, latched via `movedRef` for the rest of the drag - avoids a visible jump-then-revert on a sub-threshold jiggle); `up` separately re-checks the SAME threshold measured FRESH at the release position (not the latch) before committing - a drag that goes out past threshold and back near its origin before release still commits nothing.
- `timeline/CameraLane.tsx` - the keyframe diamond has the same two-check split as `TrimOverlay`: `move`'s live snapped-position update is latch-gated (`movedRef`), `up`'s `update_camera_move` commit is measured fresh at release (X axis alone, same as `TrimOverlay`).
- `stage/CamDragHandle.tsx` - the Move-mode PiP drag, passing its own historical `4` for `px` instead of the default; a single latch (no separate release-time re-check - this is release-agnostic, since it just decides when to start writing `camDraftRef`, not whether to commit an op).
- `stage/useReticleDrag.ts` (review round 1 minor) - the zoom-aim reticle drag withholds its first `onAimAt` commit (and the `liveAim` visual follow) the same way, using the default threshold - a 1px jiggle no longer fires a full `update_zoom` + undo step. Latched for the rest of the gesture once crossed, since this is a continuous debounced follow (not a single release-gated commit like the drags above), so there's no separate "measure at release" step here.
