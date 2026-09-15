# src/editor/director/review/ReviewSheet.tsx

The AI director's proposals, reviewed in the AI panel's own body. This is the M4 replacement for the fake-pointer performance: the director proposes, the user accepts or skips, and only then does anything reach the doc (binding decision 3).

**It is a panel, not a modal.** The sheet renders inside `AiPanel`, so the stage, the transport and the timeline all stay live underneath it - a user can scrub and play while deciding. Nothing about it takes over the screen, and the only thing it draws outside the panel is the stage outline (`StageOutline.tsx`).

## ReviewSheet

```tsx
export function ReviewSheet({ run, skipped, applying, previewId, onToggle, onPreview, onApply, onDiscard }: {
  run: AiRun; skipped: ReadonlySet<string>; applying: boolean; previewId: string | null;
  onToggle: (id: string) => void; onPreview: (id: string) => void;
  onApply: () => void; onDiscard: () => void;
}): JSX.Element
```

### Layout

1. **Header** - the model's short name (`engineDisplayName`, raw id on hover) and, at the right, what the run cost.
2. **Summary** - `summary(run, skipped)`: the count and how many are accepted, or the empty-run sentence.
3. **The list** - one `ReviewItem` per proposal, inside an `AnimatePresence` so a row that leaves fades rather than vanishing.
4. **Footer** - `Apply N edits` (primary, disabled while applying or with nothing accepted) and `Discard`.

### Honesty

`provenance` prints `"9 frames, 12.4s"` - but a run with `frames === 0` prints only the elapsed time. Saying "0 frames" would present a text-only pass as perception that happened and returned nothing, when in fact the model never saw the picture at all.

An empty run shows the sentence `"Nothing worth editing was found in this clip."` at hint size with no list, no box and no Apply - only Discard, because there is nothing to apply and nothing to look at.

### Used by

- `src/editor/panels/AiPanel.tsx` - renders it in place of the panel's what-it-does copy whenever `run` is non-null.
