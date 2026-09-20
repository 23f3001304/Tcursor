# src/editor/hooks/stage/useSyncRefs.ts

The one home of the stage's latest-value refs (Batch 4 T0). Mirrors a bag of live values into `useRef`s every render, so `useStageEngine`'s single long-lived `rAF` loop (in `useCompositeLoop`) always reads current values without re-subscribing or restarting the loop on every render.

## useSyncRefs

```ts
export function useSyncRefs({
  playing, timeMs, onTime, onSeek, track, layout, clicks, effects, clickfx, grade,
  captions, texts, capStyle, accent, cursorKinds, cursor,
  layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, arranging, map,
  dissolves, motionEasing,
}: { /* one field per Stage prop, or engine-derived value, the composite loop needs live */ }): {
  playRef, timeRef, onTimeRef, onSeekRef, trackRef, layoutRef, clicksRef, effectsRef, clickfxRef, gradeRef,
  captionsRef, textsRef, capStyleRef, accentRef, kindsRef, cursorRef,
  layoutPresetsRef, layoutSegsRef, cameraMovesRef, zoomsRef, zoomSettingsRef, arrangingRef, mapRef,
  dissolvesRef, motionEasingRef,
}
```

### Inputs

The current-render values of `Stage`'s playback/draw-affecting props, plus three values `useStageEngine` derives before calling this hook rather than passing straight through: `layoutPresets` (`arrange.presets`, the drag-draft-merged presets from `useArrangeDrag`), `arranging` (`p.arrangeSeg !== null`) and `dissolves` (`clipDissolves(p.map, p.clips)`). `layoutSegs`, `cameraMoves`, `zooms`, `zoomSettings`, `map` and `motionEasing` are plain `p.x` passthroughs, like the original fifteen.

### Returns

One `RefObject` per input, each set to the latest value on every render (`ref.current = value`, unconditionally - not inside an effect, so it's synchronous with render, not a frame behind).

### Notes

- Plain `useRef` + direct assignment, not `useEffect` - the assignment must happen during render so the very next `rAF` tick (which can fire before effects run) already sees the new value.
- `captions` / `capStyle` / `accent` (M5 T5) are the caption track on the OUTPUT clock, `doc.settings.captions` and `doc.settings.ui.accent` - the three inputs `captionDraw.drawCaptions` needs, mirrored here for the same reason the rest are.
- `texts` is the animated text track, also on the OUTPUT clock. It sits beside `captions` because the tick reads the two at the same instant and paints them one after the other; `accentRef` beside it is the second half of what `laidTexts` needs, so text costs one ref rather than three.
- `layoutPresets`, `layoutSegs`, `cameraMoves`, `zooms`, `zoomSettings`, `arranging` and `map` (Batch 4 T0) used to be six hand-rolled `useRef` + assign pairs plus one more inside `useStageEngine.ts` itself, in addition to this hook's fifteen. They moved here, unchanged in semantics, so the engine has exactly one place that owns latest-value refs instead of three - ahead of Batch 4 adding a second video element and more live refs to the same loop.
- `onSeek` (Batch 4 T6) is `Stage`'s new prop of the same name, the editor's scrub callback. It is mirrored here rather than called through a prop because its one caller is the rAF loop, which reads everything through this bundle: it stops playback at the end of a reordered take, where the editor's own source-time end rule never fires. See `useCompositeLoop.md` and `../../stage/transport/outClock.md`.
- `dissolves` and `motionEasing` (Batch 4 T7) are the clip dissolve's two live inputs: the resolved `ClipDissolve[]` the engine memoises from the time map and `doc.clips`, and `doc.settings.motion.easing`. The tick reads them together, once, through `clipMixAt`, so they sit beside each other for the same reason `texts` sits beside `captions`. The list is mirrored rather than recomputed in the loop because it costs a walk of the whole segment list, and rather than raw `clips` because the loop has no business owning the plan arithmetic. See `../../stage/clips/clipDissolve.md`.
- `useStageEngine.ts` is the only caller. It keeps the returned bundle as one object and spreads it into `useCompositeLoop`'s `CompositeLoopRefs`; `playRef`, `mapRef`, `layoutRef`, `trackRef`, `timeRef` and `onTimeRef` are also read a second time directly off that same bundle, by `useStageInvalidation`, `useMediaPlayback` and the engine's own return value.
