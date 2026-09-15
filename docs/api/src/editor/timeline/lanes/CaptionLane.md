# src/editor/timeline/lanes/CaptionLane.tsx

The Captions lane's own presentation logic for `Timeline.tsx`: the doc's caption track laid into rows, the pill's shortened label, and the `RegionRows.renderLabel` render-prop that draws it. Split out of `Timeline.tsx` for the same reason `LayoutLane.tsx` was - that file is at its 200-line cap and a fourth lane had to cost it almost nothing.

The lane sits between FX and Layout: a caption is a decoration over the picture, like a spotlight, but one whose pills get retimed far more often than a layout segment, so it belongs above Layout rather than below it.

## CaptionRegion

```ts
export type CaptionRegion = Caption & { layer: number };
```

A caption after row assignment. Captions normally never overlap - `asr::group::group_words` emits them end to end - but a hand-dragged one can, and a caption hidden under another would be unreachable on the timeline, so the lane stacks exactly like the zoom and FX lanes.

## PILL_CHARS

```ts
export const PILL_CHARS = 22;
```

How many characters of a caption reach its pill. Deliberately short: the pill is a handle for dragging, not a place to read the line, and the full text is one click away in the inspector. The pill's own container query hides the label entirely below `CAPTION_LABEL_MIN_PX` (`timeline.css`), so this only governs the wide case.

## CAPTION_LABEL_MIN_PX

```ts
export const CAPTION_LABEL_MIN_PX = 56;
```

The width a caption pill has to reach before it draws its label at all; below it the pill is its tint and nothing else. The rule itself is a container query in `timeline.css` - the pill's OWN inline size, so it reacts as a live drag-resize crosses the threshold, with no observer and no measured track width - and this constant is what the test asserts that query against.

56 rather than the 46 every other lane uses: the other lanes' labels are a glyph and a short token ("2.5x", "Spotlight"), while a caption's is prose, and under about seven characters an ellipsised line says less than the bare tint does.

## useCaptionLaneRegions

```ts
export function useCaptionLaneRegions(captions: Caption[]): CaptionRegion[]
```

`doc.captions` through `layers.ts`'s `layoutRegions`, memoized on the doc's own array - not a fresh one per render - so `useLaneDrag`'s `beginDrag` and `RegionRows`' `React.memo` both survive a playhead tick.

### useCaptionLaneRegions.pure

```ts
useCaptionLaneRegions.pure = (captions: Caption[]) => CaptionRegion[]
```

The un-memoized function hung off the hook as a property, so the row assignment can be asserted in a plain unit test with no renderer - the same escape hatch the timeline's other pure helpers give their tests. Nothing in the app calls it; production code always goes through the hook.

## captionPreviewText

```ts
export function captionPreviewText(text: string, max: number): string
```

The caption's text trimmed and shortened to at most `max` characters WITHOUT cutting a word in half: the cut is the last space at or before `max`. The one case that does get cut mid-word is a first word longer than `max`, where there is no earlier break to take.

Blank text returns the empty string rather than an ellipsis. A caption with nothing in it - the right-hand half of a split whose text had no space to cut at (`edit::ops::captions::split_text`) - should read as empty, not as a caption whose text is merely too long to show.

### Behaviors

- `shortens the pill label without cutting a word in half` - `("hello there world", 11)` is `"hello there"`; a text already within the limit is returned as is.
- `says nothing at all rather than showing an ellipsis on an empty caption` - whitespace-only text is `""`.

## captionLabel

```tsx
export const captionLabel = (c: CaptionRegion) => string
```

`RegionRows.renderLabel` for this lane: `captionPreviewText(c.text, PILL_CHARS)`, and nothing else.

It used to open with a 12px `IconBadgeCc`, and that is what the owner was looking at on 2026-09-15 when they called the lane confusing. On a take with 87 captions every pill sits at `MIN_PILL_PCT`, its floor, which is narrower than the pill is tall; the glyph was the only part of the label the pill's `overflow: hidden` did not eat, and a 2px upright sliver in a narrow block reads as a text caret. The lane already says "Captions" in the gutter and already has its own tint, so the badge was spending the pill's only readable width on the one fact nobody needed. The text is left-aligned and ellipsised now, because the first words are what tell two captions apart.

A stable MODULE-scope function, closing over nothing per render, for the same reason `zoomLabel`/`fxLabel`/`layoutLabel` are - a fresh arrow every render is a fresh prop every render, which defeats `RegionRows`' `React.memo` even when the lane's data has not changed.

### Behaviors

- `puts the caption text alone on the pill, with no icon to shear into a caret`.
- `hides the pill's label below the caption lane's own width rung, not the shared one` - the container query in `timeline.css` matches `CAPTION_LABEL_MIN_PX`, and the caption rule sits after the shared 46px one it outranks.
- `wears the audio hue, so the speech track reads with the waveforms` - `.e-capblk` and `.e-lane-captions` both take `--e-wave`.

## captionTitle

```tsx
export const captionTitle = (c: CaptionRegion) => string | undefined
```

`RegionRows.titleOf` for this lane: the caption's full text as the PILL's own `title`, so hovering any pill - including the ones too narrow to show a label at all - says what was said there.

It goes on the pill rather than on the label because `.e-zlabel` is `pointer-events: none`, and it is a native `title` rather than the editor's `Tooltip` because `Tooltip` wraps its child in a span, and 87 absolutely-positioned pills cannot each grow a wrapper without losing their positioning.

Empty text returns `undefined`, not `""`: a blank tooltip that opens on hover is worse than none.

### Behaviors

- `carries the full line as the pill's own tooltip, and none at all when it is empty`.
