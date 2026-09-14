# src/editor/timeline/CutOverlay.tsx

The stretches a cut removes, hatched across the whole track stack - `TrimOverlay`'s idiom one plane down, and for the same reason: what is gone is gone from every lane at once, not from the Time lane alone. Rendered by `Timeline.tsx` inside `.e-trackswrap`, as a sibling of `.e-tracks`.

## CutOverlay

```tsx
export const CutOverlay: React.MemoExoticComponent<(props: {
  cuts: Cut[]; dur: number; sel: string | null; onSel: (id: string | null) => void;
}) => JSX.Element | null>
```

One `.e-cut` div per cut, absolutely positioned at that cut's own share of the clip. `null` when `dur <= 0`.

### Props

- `cuts: Cut[]` - `doc.cuts`, on clip time like every lane.
- `dur: number` - the clip's duration; each span renders at `left: (start/dur)*100%`, `width: ((end-start)/dur)*100%`, with both edges clamped into `[0, dur]` so a cut that outruns a shortened clip still draws inside the track.
- `sel` / `onSel` - the editor's one selection. A `pointerdown` `stopPropagation`s (so `.e-tlbody`'s scrub does not also fire and chase the playhead) and selects the cut's id.

### Behavior

**Selection, not dragging.** Clicking a span selects it, which opens `CutInspector` and arms Delete through the shared delete-selection branch in `useEditorKeymap` (`remove_cut`). Nothing here drags: a cut's edges are edited numerically in the inspector, where a millisecond is reachable, rather than by grabbing a 2px edge on a span that may be a few pixels wide.

**Look.** `.e-cut` (timeline.css) is a PATTERN fill, not chrome decoration - the same exemption `.e-trimdim` takes from the no-gradients-on-chrome rule: 45-degree 1px `--e-dim` stripes at 18% over a 40% `--e-bg` scrim, and no border at rest. The 1px accent edges arrive only on hover, or while the cut is selected, so a resting timeline stays quiet. `z-index: 2` puts a span over the lane rows and under `.e-trimdim`'s own dimming (3), so trimmed-out time still reads as excluded even where a cut also sits inside it. A pill the span only PARTLY covers wins back the whole stack the moment its uncovered part is hovered (the existing `z-index: 7` hover bump, timeline.css). A pill a cut covers ENTIRELY is not clickable while that cut exists - which is the honest reading, since its span is time the export no longer has; shrink or remove the cut to get at it again.

**Motion.** Enter and leave only - opacity, 0.14s, inside `AnimatePresence`, so a cut applied from the transport or from Remove silences fades in rather than appearing between frames. `useReducedMotion` drops both to zero duration.

**Title.** `Cut: 1.0 s. Click to select, Delete to remove` - the duration is what a user actually wants confirmed before removing one.

### Why a sibling of the scroll container

The spans size against `.e-trackswrap`, which is `position: relative` and exactly as tall as `.e-tracks`, rather than against `.e-tracks` itself. Inside `.e-tracks` they would be absolutely-positioned children of a SCROLL container: `bottom: 0` resolves against the visible (padding-box) height, not the scroll height, and the element then scrolls with the content - so on a stack tall enough to scroll, the hatch would slide off the bottom lanes as soon as the user scrolled. As a sibling of the same box it always covers the whole lane viewport, whatever the scroll position. This is the same class of fix, for a different reason, as `.e-lanegutter` being a sibling rather than a descendant (see `Timeline.md`, "Why a sibling gutter").

### Notes

- The cut is still recording: scrubbing into a hatched span shows what it holds, and the transport's readout shows the cut's end on the output clock. Only the export and continuous playback skip it.
