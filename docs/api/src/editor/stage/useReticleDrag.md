# src/editor/stage/useReticleDrag.ts

The on-stage zoom-aim reticle's debounced, optimistic drag - extracted out of `Stage.tsx` purely
to stay under its line budget (render hygiene pass, fix round 2).

## useReticleDrag

```ts
export function useReticleDrag(
  aimPoint: [number, number] | null,
  onAimAt: (x: number, y: number) => void,
  targetUnderPointer: (clientX: number, clientY: number) => [number, number] | null,
): { liveAim: [number, number] | null; aimDrag: boolean; onReticleDown: (e: React.PointerEvent) => void }
```

### Inputs

- `aimPoint: [number, number] | null` - `Stage`'s own prop, forwarded straight through: the
  doc-driven aim point (`null` when nothing selected or the zoom follows the cursor).
- `onAimAt: (x, y) => void` - `Stage`'s own prop; the commit, debounced below.
- `targetUnderPointer` - `Stage`'s own pointer-to-screen-fraction mapper (needs `canvas`/`screen`
  refs and the live camera transform, all of which stay in `Stage` - passed in as a function
  rather than duplicating that state here).

### Returns

- `liveAim` - the live, optimistic aim point while dragging (or trailing the debounce), `null`
  when not overriding `aimPoint`. `Stage` uses `liveAim ?? aimPoint` to position the reticle.
- `aimDrag` - whether the reticle is currently under the pointer; `Stage` passes it to
  `ZoomReticle`'s `dragging` prop.
- `onReticleDown` - wire to the reticle's `onPointerDown`.

### Behavior

`onAimAt` is a full `apply_edit_op` round trip, so it's debounced like `Slider.tsx` debounces
`onChange`: `liveAim` drives the reticle at full pointer rate while the commit trails by 80ms
(`AIM_DEBOUNCE_MS`, via `debounce.ts`), flushed on release. `liveAim` clears via the shared
`shouldClearOverride` (`../util/overrideClear.ts`, also used by `Slider.tsx`) once `aimPoint`
differs from `settledAimRef.current` (its value as of the moment the drag began) - not once it
lands back on the exact point sent, which could never resolve a commit `aimAt` no-ops on
(deselecting before the round trip lands) or one `applyOp` swallows the error of. The module-level
`aimPointEq` treats `null` as a real, resolvable value (equal only to another `null`), so a
deselect mid-drag correctly clears the override too, instead of leaving a ghost reticle with
nothing selected.

`onReticleDown` attaches the drag via `attachPointerGesture` (`./pointerGesture.ts`, tested) -
`onEnd` fires on `pointerup` OR `pointercancel`, so a cancelled sequence (palm rejection, an OS
gesture stealing the pointer) can't leave `aimDrag` stuck `true` forever (which would permanently
block `shouldClearOverride`'s gate - a ghost reticle) or leak the window listeners. Its returned
`detach` is stashed in `detachRef` and called from a `useEffect` unmount cleanup (bug-sweep-2 Task
8, L4) - `attachPointerGesture` itself only tears its listeners down on `pointerup`/`pointercancel`,
neither of which fires if `Stage` unmounts mid-drag, so without this the gesture's window listeners
(and a now-orphaned closure over stale `aimPoint`/`targetUnderPointer`) would leak forever.

**Threshold before the FIRST commit (review round 1 minor).** `onMove` withholds BOTH `setLiveAim`
and the debounced `onAimAt` call until the pointer has moved past `pastDragThreshold`'s default 3px
from `onReticleDown`'s own start position - latched (`moved`) for the rest of the gesture once
crossed, since this is a continuous debounced follow, not a single release-gated commit like the
timeline drags (`useRegionDrag`/`TrimOverlay`/`CameraLane`) - there's no separate "measure at
release" step to unify with. Before this, even a 1px jiggle scheduled (and, on release, `flush()`ed)
a full `update_zoom` + undo step - the same "ANY commit needs a real drag" mandate the timeline
drags already got (D-Medium M8).

A pending commit still `flush()`es on unmount, so a value the user actually dragged to is never
silently dropped - and `flush()` on a debounce that never actually fired (a sub-threshold gesture)
is itself a safe no-op (`debounce.ts`'s `run()` guards on `pending`).

### Used by

`Stage` (`src/editor/stage/Stage.tsx`) - `const { liveAim, aimDrag, onReticleDown } = useReticleDrag(aimPoint, onAimAt, targetUnderPointer);`.
