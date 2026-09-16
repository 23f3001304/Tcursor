# src/editor/timeline/lanes/TextLane.tsx

The Text lane's regions, its pill label and its hover title. Built exactly like `CaptionLane.tsx`, and for the same reason: the lane's pills carry PROSE, so what a pill shows and how it degrades when it gets narrow are decisions worth keeping out of `useTimelineLanes`.

The lane sits between FX and Captions, which mirrors the order the frame is painted in (spec 4 fit point 6: text goes on top of the effects and under the captions), so the timeline reads top to bottom the way the frame stacks back to front.

## TextRegion

```ts
export type TextRegion = TextItem & { layer: number };
```

A text item with the ROW it was assigned. `layer` here is display only: a `TextItem` has no stored layer (spec 5.1 leaves it out deliberately) and `laidTexts` paints in array order regardless, so the row is purely so two overlapping items can both be seen and grabbed. This is exactly what `TimeLane` does for speed spans, and why `onCommitText` below ignores the layer argument `useLaneDrag` hands it.

## TEXT_PILL_CHARS

```ts
export const TEXT_PILL_CHARS = 18;
```

How much of the line a pill shows before it is cut. Shorter than the caption pill's 22 because a text pill also carries a leading glyph, and because a title is usually a handful of words where a caption is a sentence.

## useTextLaneRegions

```ts
export function useTextLaneRegions(texts: TextItem[]): TextRegion[]
```

Memoised `layoutRegions(texts)` - the rows, recomputed only when the array identity changes, so a playhead tick does not re-row the lane.

`useTextLaneRegions.pure` is the same function without the hook, which is what the tests call. The hook wrapper exists only so the memo lives at the lane's own call site.

### Behaviors

- `stacks overlapping items onto separate rows` - two overlapping items land on rows 0 and 1, and a third that overlaps neither goes back to row 0.

## textPreviewText

```ts
export function textPreviewText(text: string, max: number): string
```

The pill's line: trimmed, and cut at the last space before `max` rather than mid-word.

**An empty line returns `"Text"`, not `""`.** A text item with no words yet is exactly what the user has just added and is about to type into, and a pill with nothing in it reads as a rendering bug. The caption pill has no such case, because a caption always arrives with words.

### Behaviors

- `labels a pill with the first characters of the line` - a short line is unchanged, a long one is cut to at most `TEXT_PILL_CHARS`.
- `gives an untitled item something to show rather than an empty pill`.

## textLabel

```ts
export const textLabel: (t: TextRegion) => JSX.Element
```

The pill's contents: a 12px `IconTypography` and then `textPreviewText`.

It keeps its leading glyph where `captionLabel` dropped its CC badge, and that is a considered difference: a text item's whole line can be one word (`"128"` for a stat), so the glyph is what says "this is a text item" when the words alone will not.

It is a MODULE-SCOPE constant rather than a closure built in the lane, so its identity is stable across renders and the lane's memo survives a playhead tick - the same reason `captionLabel` is one.

## textTitle

```ts
export const textTitle: (t: TextRegion) => string | undefined
```

The pill's `title` attribute: the item's full untrimmed-by-width line, or `undefined` when it is blank so the browser shows no empty tooltip. It is what lets a pill too narrow for any label still say what is written there.
