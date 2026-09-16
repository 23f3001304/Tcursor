# src/editor/stage/mask/useStageMask.ts

The one line `useStageEngine` needs to have a mask drag box: which region the box is on, and whether this pointer mode owns the stage at all.

**Why it is its own file.** `useStageEngine` is the stage's composition root and sits at 190 of its 200-line budget; the selection lookup and the three-way mode gate are mask logic, not engine logic, and putting them here keeps the engine a list of calls rather than a place where one feature's rules live. It is the same reasoning that keeps the drag itself out of `Stage.tsx`.

## useStageMask

```ts
export function useStageMask(p: StageProps, canvasW: number, canvasH: number, tOut: number,
                             dirtyRef: RefObject<boolean>, arranging: boolean):
  ReturnType<typeof useMaskDrag>
```

Resolves the selected mask and calls `useMaskDrag` with it, returning that hook's result unchanged.

**Two decisions live here and nowhere else.**

1. **Which region.** `p.effects.find((e) => e.id === p.sel && e.kind !== "spotlight")` - the selected effect, unless it is the spotlight, which has no drag box. `null` when nothing is selected or the selection is some other lane's clip.
2. **Whether the mode is live.** `p.moveMode || arranging || p.aimMode ? null : selMask` - the mask box is the FOURTH exclusive stage pointer mode, so it stands down whenever aim, Move or arrange owns the pointer. Disabling it by passing `null` rather than by skipping the call is what keeps the hook count constant across renders; `useMaskDrag` with no effect yields no box and no live listeners.

`arranging` is passed in rather than derived because `useStageEngine` has already computed it for its own uses, and computing `p.arrangeSeg !== null` twice invites the two copies to disagree.

### Used by

- `src/editor/stage/useStageEngine.ts` - one call, whose result is returned as `mask` for `Stage` to render as `MaskOverlay`.
