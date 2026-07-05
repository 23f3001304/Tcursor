# src/editor/useSyncRefs.ts

Mirrors a bag of live prop values into `useRef`s every render, so `Stage.tsx`'s single long-lived `rAF` loop (in `useCompositeLoop`) always reads current values without re-subscribing or restarting the loop on every render.

## useSyncRefs

```ts
export function useSyncRefs({
  playing, timeMs, onTime, track, layout, clicks, effects, clickfx, cursorKinds, cursor,
}: { /* one field per Stage prop that the composite loop needs live */ }): {
  playRef, timeRef, onTimeRef, trackRef, layoutRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef
}
```

### Inputs

The current-render values of `Stage`'s playback/draw-affecting props.

### Returns

One `RefObject` per input, each set to the latest value on every render (`ref.current = value`, unconditionally - not inside an effect, so it's synchronous with render, not a frame behind).

### Notes

- Plain `useRef` + direct assignment, not `useEffect` - the assignment must happen during render so the very next `rAF` tick (which can fire before effects run) already sees the new value.
- `Stage.tsx` and `useCompositeLoop.ts` are the only consumers; this hook has no logic of its own beyond the mirroring.
