# src/editor/director/review/ReviewItem.tsx

One proposed edit in the review sheet: a switch, the kind's glyph, when it lands, why the model proposed it, and an eye that previews it.

## HINT_MOTION

```ts
export const HINT_MOTION = {
  initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 },
  transition: { duration: 0.14 },
};
```

The panel's existing enter for a row that appears in response to something - the AI progress bar and the error line in `AiPanel.tsx` already use exactly this. Exported so the sheet's own enter and its rows share one motion, rather than two hand-written tweens drifting apart. Suppressed entirely under `useReducedMotion`.

## ReviewItem

```tsx
export function ReviewItem({ p, accepted, previewing, disabled, onToggle, onPreview }: {
  p: AiProposal; accepted: boolean; previewing: boolean; disabled: boolean;
  onToggle: (id: string) => void; onPreview: (id: string) => void;
}): JSX.Element
```

### Behavior

**Accepted is the resting state.** The switch starts on, so doing nothing and pressing Apply accepts the whole run - what a user who trusts the result expects. A skipped row stays fully legible at reduced opacity; it is out of the run, not deleted from it.

**One line for the reason.** `.e-rev-why` ellipsises and the `title` carries the kind, the time and the whole sentence, so nothing is lost. Six wrapped paragraphs would outgrow the 320px panel, which is the budget every panel in this editor is held to.

**`disabled`** is the sheet's `applying` flag: while the accepted items are landing, neither control responds, so a second Apply cannot race the first and the run cannot change under it.

**Glyphs** are the ones this editor already uses: the Effects panel's zoom, the rail's layout grid, the wand for a spotlight, the transport's trim and cut, and the fast-forward arrow for a sped-up stretch. An unknown kind falls back to the wand rather than rendering a hole.

### Used by

- `src/editor/director/review/ReviewSheet.tsx` - one per proposal.
