# src/editor/shell/ShellStage.tsx

## ShellStage

```tsx
export function ShellStage({ p }: { p: ShellProps }): JSX.Element
```

The stage column of the classic shell - `.e-stagetoast` and its three children: the `Stage` itself, the undo `Toast` and the director's `DirectorOverlay`. Split out of `ClassicShell.tsx`, which keeps the composition around it (rail, animated panel wrap, this column, animated properties side, then `Transport` and `Timeline` underneath); the thirty-odd props `Stage` takes are all derived from `ShellProps`, so the split is one line at the call site and no new plumbing.

It takes the whole `p: ShellProps` bag rather than its own prop list, the same way `PropertiesSlot` does - there is exactly one caller, every value it reads is already on that bag, and a hand-written prop list would be a second place to update whenever `Stage` grows a prop.

### What it derives

Everything here is spread off `p` rather than carried as its own `SlotProps` field, which is why the caption renderer needed no change to `slotProps.ts`: `Stage` gets `captions={p.outDoc.captions}` (the track on the OUTPUT clock, beside `effects` and `layoutSegs`), `texts={p.outDoc.texts}` (the OUT doc again, because the text layout runs on output time), `capStyle={p.doc.settings.captions}` and `accent={p.doc.settings.ui.accent}` straight off the doc. The colour grade arrives the same way, as `grade={p.doc.settings.grade}` beside `clickfx`: it is a doc SETTING and not a region, so there is nothing on `outDoc` to read and nothing for `remapDoc` to move onto the output clock. `sel={p.sel}` is the shell's current selection id, and it is what turns a mask region into a drag box: `useStageMask` looks the id up in `effects` and hands the region to `useMaskDrag`, so with nothing selected the stage has no fourth pointer mode at all. The AI review sheet's `stageOutline` arrives the same way, as `Stage`'s `outline` prop - the shell decides nothing about it (see `ClassicShell.md`). `onSeek={p.onSeek}` (Batch 4 T6) is the newest of these one-liners: the shell's own scrub callback, already on the bag for the timeline and the inspectors, handed to the Stage as well so the composite loop can stop playback at the end of a reordered take (`stageProps.md`). It needed no new `SlotProps` field and no line in `Editor.tsx`, which is the whole reason the loop asks for the callback rather than for a second way of reporting the time. `clips={p.doc.clips}` and `motionEasing={p.doc.settings.motion.easing}` (Batch 4 T7) arrive the same way, and from the DOC rather than `outDoc` on purpose: `remapDoc` empties the clip list because the time map has already flattened it, and the dissolve's clip indices are indices into the list the map was built from. Like the grade beside them, the motion easing is a doc SETTING with nothing on `outDoc` to read.

### Why the toast and the overlay live here

Both are positioned against the stage, not against the window: the undo toast sits over the picture it is offering to undo, and the director's overlay dims and labels the same rectangle it is driving. Keeping the three in one element means the column's own width - whatever the two animated side columns leave it - is the one box all three are laid out in.
