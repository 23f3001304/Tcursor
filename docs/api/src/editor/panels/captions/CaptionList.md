# src/editor/panels/captions/CaptionList.tsx

The transcript, in the Captions panel: one row per caption, its start time and what it says.

## CaptionList

```tsx
export function CaptionList({ captions, timeMs, sel, onPick }: {
  captions: Caption[]; timeMs: number; sel: string | null; onPick: (c: Caption) => void;
}): JSX.Element
```

A row is a time and a line, and the line is the row's whole reason to exist. It used to be one clipped line with a native `title`, which on the owner's 87-caption take meant hovering 87 times to read a transcript that was already on screen (2026-09-15). Now the text wraps to at most two lines (`-webkit-line-clamp: 2`, `panels.css`) on 8px of vertical padding, and there is no tooltip at all: nothing is hidden, so nothing needs revealing. Two lines rather than all of them because a row still has to be scannable as a row; a caption long enough to clip is long enough that the inspector is the right place to read it.

The time is a small dim mono label of fixed width, so every line below it starts on the same column.

### Two "current" states

- **live** - the caption the playhead is inside, which is the one the stage is drawing right now. Decided by `captionAt`, the renderer's OWN function, imported rather than re-derived: a list that lit a different row than the frame paints would be a second opinion about the same fact. It takes the captions lane's own tint at 24% over the raised plane - the same mix the timeline's selected pills use, so "this is the caption in play" looks the same in both places.
- **on** - the selected caption, the one `CaptionInspector` is editing. Takes the raised plane and a strong hairline.

They are usually the same row, and `.e-caplist-row.live.on` is the last rule in the sheet, so a row that is both keeps the tint AND the line: one row, two marks, never a third appearance.

### Following the playhead

An effect keyed on the LIVE caption's id (not on `timeMs`) scrolls that row into view with `block: "nearest"`. Keyed on the id because a scroll on every playhead tick would fight the hand that is scrolling the list, and `"nearest"` because a row already in view should not move at all. The row is held by a ref that is attached only to the live row, so there is no id-to-element lookup to escape or keep in sync.

`timeMs` is CLIP ms, the clock `doc.captions` is stored on (the stage evaluates `outDoc`, the remapped copy). `ClassicShell` passes the live playhead to `EditorPanels` on the `captions` tab for exactly this reason; on every tab but this one and `camera` it passes a constant `0` so `React.memo` can still skip a tick.

### Height

The list takes whatever height the panel has left and scrolls inside itself, rather than the fixed 240px cap it used to carry. A cap left dead space under the list on a tall window and a four-row stub on a short one, and a transcript is the one thing in this editor whose length is the recording's, not the design's. `.e-cappanel` (the panel's own flex column) and `.e-capgrp`'s floor are what make "whatever is left" a real number - see `panels.css`.

### `onPick`

Wired to seek to the caption's start AND select it, which is what opens the Caption inspector on the right - the same two things clicking its pill on the timeline does. The transcript is therefore a second way INTO one editing surface, never a second editor.

### Behaviors

- `lights the row the playhead is inside and moves the mark as it plays`.
- `stacks the live mark and the selected mark on one row without dropping either` - both classes and both ARIA states on one row.
- `shows the whole line in the row rather than behind a hover tooltip` - no `title` attribute.

### Used by

- `src/editor/panels/captions/CaptionsPanel.tsx`
