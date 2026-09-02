# src/editor/stage/pointerGesture.ts

A small, tested primitive for a window-level pointer drag gesture, shared by any Stage-area drag
that isn't already using `useRegionDrag` (the timeline's own equivalent - see
`useRegionDrag.md`)'s pattern.

## GestureTarget

```ts
export interface GestureTarget {
  addEventListener(type: string, cb: (e: PointerEvent) => void): void;
  removeEventListener(type: string, cb: (e: PointerEvent) => void): void;
}
```

The minimal DOM-ish surface `attachPointerGesture` needs - satisfied by `window` (the real
caller), or a fake recording target in tests (`pointerGesture.test.ts`), so the whole gesture
lifecycle is unit-testable without a real DOM.

## attachPointerGesture

```ts
export function attachPointerGesture(
  onMove: (e: PointerEvent) => void,
  onEnd: () => void,
  target: GestureTarget = window,
): () => void
```

Attaches a window-level pointer-drag gesture: `onMove` fires per `pointermove`, and `onEnd` fires
exactly once when the gesture is over. Returns a `detach` function.

### Behavior

`onEnd` is wired to BOTH `pointerup` and `pointercancel` - a cancelled sequence (palm rejection,
an OS gesture stealing the pointer mid-drag) never fires `pointerup` at all. Both exit paths
remove all three listeners (`pointermove`/`pointerup`/`pointercancel`) BEFORE calling `onEnd`, so
neither path can leak a listener or leave the gesture "stuck active" forever - the caller's own
"is dragging" state relies on `onEnd` always eventually firing exactly once, from whichever exit
path the pointer sequence actually takes.

**The returned `detach` (bug-sweep-2 Task 8).** Removes all three listeners WITHOUT calling
`onEnd` - neither `pointerup` nor `pointercancel` fires when the component that started the
gesture unmounts mid-drag (Move mode toggled off, Stage unmounting), so callers that care stash
the return value in a ref and call it from a `useEffect` unmount cleanup. Safe to call again after
a real end already ran (`removeEventListener` on an already-removed listener is a no-op) - see
`pointerGesture.test.ts`'s idempotency case.

### Used by

`useReticleDrag` (`./useReticleDrag.ts`) - the on-stage zoom-aim reticle's drag; also calls the
returned `detach` from an unmount effect. `CamDragHandle` (`./CamDragHandle.ts`) - the Move-mode
PiP drag; same unmount-`detach` pattern. `useRegionDrag`, `CameraLane`, and `TrimOverlay` each
still wire their own raw `window.addEventListener`/`removeEventListener` pairs directly rather
than this helper (they already tear down correctly via an effect keyed on drag-active state - see
each file's own notes - so `pointercancel`/unmount-`detach` weren't in bug-sweep-2 Task 8's scope
for them).
