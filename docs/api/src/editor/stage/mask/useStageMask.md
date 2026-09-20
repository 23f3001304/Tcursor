# src/editor/stage/mask/useStageMask.ts

The one line `useStageEngine` needs to have a mask drag box: which region the box is on, and whether this pointer mode owns the stage at all.

**Why it is its own file.** `useStageEngine` is the stage's composition root and sits at 198 of its 200-line budget; the selection lookup, the three-way mode gate and the frame resolution are mask logic, not engine logic, and putting them here keeps the engine a list of calls rather than a place where one feature's rules live. It is the same reasoning that keeps the drag itself out of `Stage.tsx`.

## useStageMask

```ts
export function useStageMask(p: StageProps, canvasW: number, canvasH: number, tOut: number,
                             dirtyRef: RefObject<boolean>, arranging: boolean):
  ReturnType<typeof useMaskDrag>
```

Resolves the selected mask and the frame it is drawn in, calls `useMaskDrag` with both, and returns that hook's result unchanged.

**Three decisions live here and nowhere else.**

1. **Which region.** `p.effects.find((e) => e.id === p.sel && e.kind !== "spotlight")` - the selected effect, unless it is the spotlight, which has no drag box. `null` when nothing is selected or the selection is some other lane's clip.
2. **Whether the mode is live.** `p.moveMode || arranging || p.aimMode ? null : selMask` - the mask box is the FOURTH exclusive stage pointer mode, so it stands down whenever aim, Move or arrange owns the pointer. Disabling it by passing `null` rather than by skipping the call is what keeps the hook count constant across renders; `useMaskDrag` with no effect yields no box and no live listeners.
3. **Which frame.** `maskFrameAt(p, tOut, canvasW, canvasH, null)` - **the overlay shares the painter's frame.** `drawCompositeFrame` paints the mask with the layout-segment-and-camera-move-resolved `frameLayout` and the live `camAt(track, tOut)`; this hook now hands `useMaskDrag` the same pair, from the same function, so the box and the blur are one projection rather than two. It used to pass `p.layout` and a hard-coded `{ cx: 0.5, cy: 0.5, scale: 1 }`, which put the box a factor of the live zoom away from the blur - and the drag arithmetic with it, since `onHandleDown` derives its pixels-per-fraction from the same geometry.

The pair is `useMemo`d on the ten values `maskFrameAt` reads - the track, the layout and its presets and segments, the camera moves, the zooms and their settings, `tOut` and the canvas size - so it is one stable object per instant rather than a new one per render: `onHandleDown`'s dependency list holds `layout` and `cam`, and a fresh identity each render would rebuild the callback under the pointer.

*Why the drag pose is `null`.* The hook carries an `INVARIANT` block saying it: the overlay is off in move mode and while arranging, which are the only two states in which the painter's `activeCamDraft` is not `null`. There is no instant at which the overlay is on screen and a pose is live.

`arranging` is passed in rather than derived because `useStageEngine` has already computed it for its own uses, and computing `p.arrangeSeg !== null` twice invites the two copies to disagree.

### Used by

- `src/editor/stage/useStageEngine.ts` - one call, whose result is returned as `mask` for `Stage` to render as `MaskOverlay`.
