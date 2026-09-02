import { useEffect, useRef, useState } from "react";
import { debounce } from "../hooks/debounce";
import { shouldClearOverride } from "../hooks/overrideClear";
import { attachPointerGesture } from "./pointerGesture";
import { pastDragThreshold } from "../hooks/dragThreshold";

// Same trailing-debounce window Slider.tsx uses for its onChange->IPC commit - the reticle drag
// calls `onAimAt` (a full `apply_edit_op` round trip) once per pointermove otherwise.
const AIM_DEBOUNCE_MS = 80;
// `shouldClearOverride`'s `eq` for an aim point - `null` counts as equal only to `null`.
const aimPointEq = (a: [number, number] | null, b: [number, number] | null) =>
  (a === null && b === null) || (!!a && !!b && Math.abs(a[0] - b[0]) < 1e-9 && Math.abs(a[1] - b[1]) < 1e-9);

/** The on-stage zoom-aim reticle's debounced, optimistic drag - extracted out of `Stage.tsx`
 *  purely to stay under its line budget. See `Stage.md`'s "Reticle drag debounce" for the full
 *  write-up (the render-hygiene IPC-storm fix, the ghost-override fix, and the pointercancel fix
 *  all documented there); this file is the implementation.
 *
 *  `onAimAt` is a full `apply_edit_op` round trip, so it's debounced like `Slider.tsx` debounces
 *  `onChange`: `liveAim` drives the reticle at full pointer rate while the commit trails by
 *  `AIM_DEBOUNCE_MS`, flushed on release. `liveAim` clears via the shared `shouldClearOverride`
 *  (`../hooks/overrideClear.ts`) once `aimPoint` differs from `settledAimRef.current` (its value
 *  as of the moment the drag began) - not once it lands back on the exact point sent, which could
 *  never resolve a commit `aimAt` no-ops on (deselecting before the round trip lands) or one
 *  `applyOp` swallows the error of. `aimPointEq` treats `null` as a real, resolvable value (equal
 *  only to another `null`), so a deselect mid-drag correctly clears the override too, instead of
 *  leaving a ghost reticle with nothing selected. */
export function useReticleDrag(
  aimPoint: [number, number] | null,
  onAimAt: (x: number, y: number) => void,
  targetUnderPointer: (clientX: number, clientY: number) => [number, number] | null,
) {
  const [liveAim, setLiveAim] = useState<[number, number] | null>(null);
  const [aimDrag, setAimDrag] = useState(false); // the reticle is under the pointer right now
  const settledAimRef = useRef<[number, number] | null>(aimPoint);
  const onAimAtRef = useRef(onAimAt); onAimAtRef.current = onAimAt;
  const debouncedRef = useRef<ReturnType<typeof debounce<[number, number]>> | null>(null);
  if (!debouncedRef.current) debouncedRef.current = debounce((x: number, y: number) => onAimAtRef.current(x, y), AIM_DEBOUNCE_MS);
  const detachRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (shouldClearOverride(aimDrag, liveAim !== null, aimPoint, settledAimRef.current, aimPointEq)) setLiveAim(null);
  }, [aimPoint, aimDrag, liveAim]);
  useEffect(() => () => debouncedRef.current?.flush(), []); // a pending commit must still land if unmounted mid-drag
  // Release this gesture's window listeners if Stage unmounts mid-drag (L4) - `attachPointerGesture`
  // itself only tears them down on pointerup/pointercancel, neither of which fires on unmount.
  useEffect(() => () => detachRef.current?.(), []);

  // `attachPointerGesture` (`./pointerGesture.ts`, tested) wires BOTH `pointerup` and
  // `pointercancel` as "the drag is over" - without `pointercancel`, a cancelled sequence (palm
  // rejection, an OS gesture stealing the pointer) used to leave `aimDrag` stuck `true` forever,
  // permanently blocking `shouldClearOverride`'s gate (a ghost reticle) and leaking listeners.
  const onReticleDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    setAimDrag(true);
    settledAimRef.current = aimPoint; // pre-drag aim point - see the ref's own comment above
    // A 1px jiggle must not fire `onAimAt` (a full `update_zoom` + undo step) - same "ANY commit
    // needs a real drag" mandate as the timeline drags (review round 1 minor). Withheld until past
    // the threshold, THEN latched: once a real drag has started, every subsequent move (even a
    // sub-threshold one FROM THERE) keeps tracking normally - this isn't a release-gated single
    // commit like the timeline drags, it's a continuous debounced follow for the rest of the
    // gesture, so there's no separate "measure at release" step to unify with.
    const start = { x: e.clientX, y: e.clientY };
    let moved = false;
    detachRef.current = attachPointerGesture(
      (ev) => {
        if (!moved) {
          if (!pastDragThreshold(ev.clientX - start.x, ev.clientY - start.y)) return;
          moved = true;
        }
        const t = targetUnderPointer(ev.clientX, ev.clientY);
        if (t) { setLiveAim(t); debouncedRef.current!(t[0], t[1]); }
      },
      () => { detachRef.current = null; setAimDrag(false); debouncedRef.current!.flush(); },
    );
  };

  return { liveAim, aimDrag, onReticleDown };
}
