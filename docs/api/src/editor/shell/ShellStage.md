# src/editor/shell/ShellStage.tsx

## ShellStage

```tsx
export function ShellStage({ p }: { p: ShellProps }): JSX.Element
```

The stage column of the classic shell - `.e-stagetoast` and its three children: the `Stage` itself, the undo `Toast` and the director's `DirectorOverlay`. Split out of `ClassicShell.tsx`, which keeps the composition around it (rail, animated panel wrap, this column, animated properties side, then `Transport` and `Timeline` underneath); the thirty-odd props `Stage` takes are all derived from `ShellProps`, so the split is one line at the call site and no new plumbing.

It takes the whole `p: ShellProps` bag rather than its own prop list, the same way `PropertiesSlot` does - there is exactly one caller, every value it reads is already on that bag, and a hand-written prop list would be a second place to update whenever `Stage` grows a prop.

### What it derives

Everything here is spread off `p` rather than carried as its own `SlotProps` field, which is why the caption renderer needed no change to `slotProps.ts`: `Stage` gets `captions={p.outDoc.captions}` (the track on the OUTPUT clock, beside `effects` and `layoutSegs`), `capStyle={p.doc.settings.captions}` and `accent={p.doc.settings.ui.accent}` straight off the doc. The AI review sheet's `stageOutline` arrives the same way, as `Stage`'s `outline` prop - the shell decides nothing about it (see `ClassicShell.md`).

### Why the toast and the overlay live here

Both are positioned against the stage, not against the window: the undo toast sits over the picture it is offering to undo, and the director's overlay dims and labels the same rectangle it is driving. Keeping the three in one element means the column's own width - whatever the two animated side columns leave it - is the one box all three are laid out in.
