/** A DOM-ish surface `attachPointerGesture` needs - satisfied by `window` (the real caller), or a
 *  fake target in tests. */
export interface GestureTarget {
  addEventListener(type: string, cb: (e: PointerEvent) => void): void;
  removeEventListener(type: string, cb: (e: PointerEvent) => void): void;
}

/** Attaches a window-level pointer-drag gesture: `onMove` fires per `pointermove`, and `onEnd`
 *  fires exactly once when the gesture is over - on `pointerup` OR `pointercancel` (a cancelled
 *  sequence, e.g. palm rejection or an OS gesture stealing the pointer, never fires `pointerup`).
 *  Both exit paths remove all three listeners before calling `onEnd`, so neither one can leak a
 *  listener or leave the gesture "stuck active" forever - the exact bug this replaces (only
 *  `pointerup` wired, so a cancelled drag left its listeners attached and its caller's "still
 *  dragging" flag permanently `true`).
 *
 *  Returns a `detach` function that removes all three listeners WITHOUT calling `onEnd` - the
 *  caller's own unmount-safety net (e.g. a `useEffect` cleanup) for the case neither `pointerup`
 *  nor `pointercancel` ever fires because the component unmounts mid-gesture (Move mode toggled
 *  off, or the drag handle's own parent going away). Safe to call after the gesture already ended
 *  on its own - `removeEventListener` on an already-removed listener is a no-op.
 *
 *  `target` defaults to `window`; overridable for tests (a fake object recording registered
 *  listeners) without needing a real DOM. Plain function, no React. */
export function attachPointerGesture(
  onMove: (e: PointerEvent) => void,
  onEnd: () => void,
  target: GestureTarget = window,
): () => void {
  const detach = () => {
    target.removeEventListener("pointermove", onMove);
    target.removeEventListener("pointerup", end);
    target.removeEventListener("pointercancel", end);
  };
  const end = () => { detach(); onEnd(); };
  target.addEventListener("pointermove", onMove);
  target.addEventListener("pointerup", end);
  target.addEventListener("pointercancel", end);
  return detach;
}
