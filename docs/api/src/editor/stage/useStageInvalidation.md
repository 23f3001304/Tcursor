# src/editor/stage/useStageInvalidation.ts

Everything that decides WHEN the stage must recomposite, and when a live drag draft dies - four effects lifted verbatim out of `Stage.tsx` (T34 L3) purely to keep that file under its 200-line budget, exactly like `useReticleDrag` was. No behaviour changed in the move.

## useStageInvalidation

```ts
export function useStageInvalidation({ bg, bgRef, dirtyRef, drawDeps, effects, spotSimRef,
  timeMs, playRef, camDraftRef }: { ... }): void
```

`drawDeps` is every draw-affecting prop in a stable order - the dependency list of effect 2, passed as an array because the caller owns which props those are.

### The five effects

1. **Background decode.** The export background (a data URL) is decoded once per change into the `<img>` the canvas draws, marking the frame dirty when it lands. It writes into `bgRef`'s `StageBgState` (`stage/canvas/stageBg.ts`) - the single object holding everything the background draw needs, which is what let `Stage`, `useCompositeLoop` and `drawPreview` gain video backgrounds without gaining a prop.
1b. **Moving background.** When the doc's background is a video or GIF asset, this owns the thing that plays it: a GIF is decoded to frames (`gifFrames.ts`, because a `<video>` cannot play one and an `<img>` cannot be seeked), anything else gets a detached `<video muted loop playsInline>` - created with `document.createElement` and never mounted, since `drawImage` needs no DOM node. Keyed on the asset URL and path, and torn down in the cleanup, so switching back to a wallpaper STOPS the decode rather than leaving it running invisibly. Effect 4 below then keeps it in step with the transport.
2. **Dirty on any draw-affecting change.** A PAUSED rAF loop only composites when `dirtyRef` is set, so this is what makes it repaint exactly once per change instead of redrawing a static frame at 60fps (the idle/interaction-lag fix). Note the caller passes the DRAFT-MERGED presets here, so an arrange drag invalidates through the same channel every other edit does.
3. **Spotlight sim reset, gate 2.** A retime/add/remove of a Spotlight region WHILE PAUSED never moves `timeMs`, so the composite loop's own discontinuous-jump reset (gate 1) cannot fire, and an in-flight fade-out would ease forever against a frozen `t`. Keyed on spotlight CONTENT (`spotlightEffectsKey`), not the `effects` array reference - `applyEditOp` hands back a fresh array on every edit routed through it, spotlight or not.
4. **Camera draft clearing, and the background video's transport.** Moving the playhead discards the unsaved Move-mode pose - UNLESS the `timeMs` change is just the loop's own natural playback progress rather than a real seek/scrub (`isNaturalPlaybackTick`, M6). The draft must survive an ordinary tick whether the drag is still in progress or already released: "nothing is saved until you press the button" only holds if the draft lives that long. `playRef` keeps this reading the CURRENT `playing`, not whatever it was when the effect last ran.

   The same effect latches `playing` into the background state and drives the background `<video>`: it plays with the transport and, when paused, seeks to `loopMs(timeMs, duration)`. That seek is what makes scrubbing show the right background frame instead of a stopped one; during playback the element runs at 1x and only real drift is corrected, in the draw (see `stageBg.ts`), because seeking per frame would stutter the picture.

Arrange mode's own draft is NOT cleared here - it is not playhead-scoped (a segment's arrangement is the same wherever the playhead sits), and it hands over on the resolved-rect refetch instead. See `arrange/useArrangeDrag.md`.
