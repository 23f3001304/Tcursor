# src/editor/inspectors/captionEdit.ts

The pure decisions behind `CaptionInspector`. Every one mirrors a rule that Rust's `edit::ops::captions` already enforces, so the inspector can only ever offer an edit the backend will actually perform - a control that sends a no-op reads as broken. Nothing here mutates a caption: the ops do that, this file only decides what to offer and what to say about it.

## SplitPoint

```ts
export interface SplitPoint { atMs: number; label: string }
```

One place a caption may be cut, and the word that would start the second half. `label` is what goes on the chip.

## insideSpan

```ts
export const insideSpan: (cap: Caption, ms: number) => boolean
```

Strictly between the edges. Rust's `SplitCaption` is a no-op at either edge (the plan pins that: a split exactly at `start_ms` changes nothing), so a Split control offered at one would do nothing.

## splitPoints

```ts
export function splitPoints(cap: Caption): SplitPoint[]
```

Every word boundary this caption can be split at, left to right: each word AFTER the first, cut at that word's own `start_ms`, so the split always lands between words rather than inside one. A boundary outside the caption is dropped (ASR can emit a word whose start precedes its neighbour's), and an instant already offered is never offered twice - both would be Rust no-ops.

A caption with no word timings returns `[]`, and the inspector falls back to "Split at the playhead", which Rust splits at the nearest space in the text instead.

Why word boundaries rather than the playhead in the normal case: a caption cut mid-word reads as a typo in the export, and the ASR words carry the only information about where a cut is safe. Scrubbing to a word boundary by eye on a 2-second pill is not a thing anyone can do.

## wordsSurvive

```ts
export function wordsSurvive(cap: Caption, text: string): boolean
```

Would this new text keep the caption's word timings? Mirrors Rust's rule exactly: they survive only when the text IS the words joined by single spaces (leading and trailing space ignored). Anything else clears them.

**Why dropping, not redistributing.** A word-by-word highlight driven by redistributed timings lights the WRONG word - proportional redistribution assumes the new text has the same rhythm as the old, which is exactly what is untrue when someone fixes a name or rewrites a sentence. A caption with no `words` simply renders with the highlight off (`captionPreview::layoutCaption` returns `hi: null`), which is a visibly quieter caption rather than a wrong one. The inspector says which of the two an edit will be BEFORE it is committed, so losing a highlight is never a surprise.

## nextCaption

```ts
export function nextCaption(caps: Caption[], id: string): Caption | null
```

The caption `merge_captions` would join this one with: the next in TIME order, which is not the next in array order on a doc mid-edit. `null` is exactly the case where Merge must be inert (the last caption), and the inspector disables the button and says why rather than letting a press do nothing.

### Used by

- `src/editor/inspectors/CaptionInspector.tsx`
