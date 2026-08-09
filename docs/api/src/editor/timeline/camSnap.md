# src/editor/timeline/camSnap.ts

Snapping for camera-keyframe drags on the timeline's Camera lane (Task 27). Pure functions, no DOM and no doc mutation, so `CameraLane.tsx` can call them on every `pointermove` and they can be unit-tested without rendering anything.

Everything here works in MILLISECONDS, never pixels: the lane's px-per-ms changes with the window width (and with any future timeline zoom), so a px-based tolerance would snap aggressively on a narrow window and barely at all on a wide one. A ms tolerance feels identical everywhere.

## CAM_SNAP_MS

```ts
export const CAM_SNAP_MS = 80;
```

How close (ms) a dragged keyframe must get to a snap target before it locks on. The window is INCLUSIVE - a candidate exactly `80` ms away snaps; `81` does not.

Distinct from `CAM_KF_SNAP_MS` (`src/editor/stage/camKeyframeAt.ts`, `60`), which answers a different question: whether an edit at the playhead should UPDATE an existing keyframe instead of adding a new one. That one is about identity ("is this the same keyframe?"); this one is about placement ("should this drag land exactly on that edge?").

## snapKeyframeMs

```ts
export function snapKeyframeMs(t: number, segEdges: number[], otherKfs: number[]): number
```

Snaps a dragged keyframe's time to the nearest candidate within `CAM_SNAP_MS`, else returns `t` unchanged.

### Inputs

- `t: number` - the raw dragged time in ms, already clamped to `[0, dur]` by the caller.
- `segEdges: number[]` - every layout-segment boundary: `doc.layout.flatMap((s) => [s.start_ms, s.end_ms])`. *Why these:* after Task 27 a keyframe's span is what it OWNS, and the thing it competes with is the layout track - so landing exactly on a segment's start or end is the placement users actually want (hand the panel over precisely at the cut).
- `otherKfs: number[]` - the OTHER keyframes' times. The dragged keyframe's own time must be excluded by the caller, or it would snap back to where the drag started and refuse to move.

### Returns

The snapped time, or `t` unchanged when nothing is in range. Ties go to the earliest candidate in `segEdges` then `otherKfs` (both come from the doc in a fixed order, so the choice is stable frame to frame - no jitter between two equidistant targets).

### Implementation

Single pass over `[...segEdges, ...otherKfs]` tracking the best `|c - t|`, accepted only when `d <= CAM_SNAP_MS && d < bestDist`. Starting `bestDist` at `Infinity` (rather than at `CAM_SNAP_MS`) is what makes the boundary inclusive while keeping first-wins tie-breaking.

### Behaviors worth knowing

- `1930` and `2070` both snap to a segment edge at `2000` (70 ms away).
- `1920` and `2080` snap (exactly `CAM_SNAP_MS`); `1919` and `2081` do not - they come back unchanged.
- `3040` snaps to a sibling keyframe at `3000`: keyframe times are targets too, not just segment edges.
- With a segment edge at `2000` and a keyframe at `2060`, a drag at `2050` picks `2060` (10 ms beats 50 ms) while a drag at `2010` picks `2000` - nearest wins regardless of which list it came from.
- No candidates at all -> the time is returned untouched.
- Candidates far outside the window on both sides never pull the drag.

### Used by

- `src/editor/timeline/CameraLane.tsx` - inside the drag `pointermove` handler, between the px->ms conversion and `setDrag`, so the live diamond position (and the span bar that follows it) is already snapped before anything is committed on release.
