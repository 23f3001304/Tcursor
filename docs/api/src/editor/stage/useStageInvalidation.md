# src/editor/stage/useStageInvalidation.ts

Everything that decides WHEN the stage must recomposite, and when a live drag draft dies - four effects lifted verbatim out of `Stage.tsx` (T34 L3) purely to keep that file under its 200-line budget, exactly like `useReticleDrag` was. No behaviour changed in the move.

## useStageInvalidation

```ts
export function useStageInvalidation({ bgUrl, bgImgRef, dirtyRef, drawDeps, effects, spotSimRef,
  timeMs, playRef, camDraftRef }: { ... }): void
```

`drawDeps` is every draw-affecting prop in a stable order - the dependency list of effect 2, passed as an array because the caller owns which props those are.

### The four effects

1. **Background decode.** The export background (a data URL) is decoded once per change into the `<img>` the canvas draws, marking the frame dirty when it lands.
2. **Dirty on any draw-affecting change.** A PAUSED rAF loop only composites when `dirtyRef` is set, so this is what makes it repaint exactly once per change instead of redrawing a static frame at 60fps (the idle/interaction-lag fix). Note the caller passes the DRAFT-MERGED presets here, so an arrange drag invalidates through the same channel every other edit does.
3. **Spotlight sim reset, gate 2.** A retime/add/remove of a Spotlight region WHILE PAUSED never moves `timeMs`, so the composite loop's own discontinuous-jump reset (gate 1) cannot fire, and an in-flight fade-out would ease forever against a frozen `t`. Keyed on spotlight CONTENT (`spotlightEffectsKey`), not the `effects` array reference - `applyEditOp` hands back a fresh array on every edit routed through it, spotlight or not.
4. **Camera draft clearing.** Moving the playhead discards the unsaved Move-mode pose - UNLESS the `timeMs` change is just the loop's own natural playback progress rather than a real seek/scrub (`isNaturalPlaybackTick`, M6). The draft must survive an ordinary tick whether the drag is still in progress or already released: "nothing is saved until you press the button" only holds if the draft lives that long. `playRef` keeps this reading the CURRENT `playing`, not whatever it was when the effect last ran.

Arrange mode's own draft is NOT cleared here - it is not playhead-scoped (a segment's arrangement is the same wherever the playhead sits), and it hands over on the resolved-rect refetch instead. See `arrange/useArrangeDrag.md`.
