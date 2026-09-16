# src/editor/hooks/stage/useSyncRefs.ts

Mirrors a bag of live prop values into `useRef`s every render, so `Stage.tsx`'s single long-lived `rAF` loop (in `useCompositeLoop`) always reads current values without re-subscribing or restarting the loop on every render.

## useSyncRefs

```ts
export function useSyncRefs({
  playing, timeMs, onTime, track, layout, clicks, effects, clickfx, grade,
  captions, texts, capStyle, accent, cursorKinds, cursor,
}: { /* one field per Stage prop that the composite loop needs live */ }): {
  playRef, timeRef, onTimeRef, trackRef, layoutRef, clicksRef, effectsRef, clickfxRef, gradeRef,
  captionsRef, textsRef, capStyleRef, accentRef, kindsRef, cursorRef
}
```

### Inputs

The current-render values of `Stage`'s playback/draw-affecting props.

### Returns

One `RefObject` per input, each set to the latest value on every render (`ref.current = value`, unconditionally - not inside an effect, so it's synchronous with render, not a frame behind).

### Notes

- Plain `useRef` + direct assignment, not `useEffect` - the assignment must happen during render so the very next `rAF` tick (which can fire before effects run) already sees the new value.
- `captions` / `capStyle` / `accent` (M5 T5) are the caption track on the OUTPUT clock, `doc.settings.captions` and `doc.settings.ui.accent` - the three inputs `captionDraw.drawCaptions` needs, mirrored here for the same reason the rest are.
- `texts` is the animated text track, also on the OUTPUT clock. It sits beside `captions` because the tick reads the two at the same instant and paints them one after the other; `accentRef` beside it is the second half of what `laidTexts` needs, so text costs one ref rather than three.
- `Stage.tsx` and `useCompositeLoop.ts` are the only consumers; this hook has no logic of its own beyond the mirroring.
