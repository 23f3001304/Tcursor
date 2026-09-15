# src/editor/timeline/useLaneDrag.ts

One pill lane's drag state plus its row count, in one call. Extracted from `Timeline.tsx` (M5 T4, plan ADDED-7), where the same three-line block - `useRegionDrag`, then `Math.max(...regions.map(r => r.layer), drag?.layer ?? 0)`, then `regions.length ? max + 1 : 0` - was written out once per lane. Collapsing it is what made room for a fourth lane under that file's 200-line cap.

A pure extraction: the arithmetic is identical, including the `drag` term in the max, which is the part that is easy to lose and load-bearing - it is what lets a pill dragged UP off the top row open its new row LIVE, while the pointer is still down, instead of only once the commit lands.

## useLaneDrag

```ts
export function useLaneDrag<T extends { id: string; start_ms: number; end_ms: number; layer: number }>(
  regions: T[], dur: number, trackRef: React.RefObject<HTMLDivElement | null>, rowHeightPx: number,
  onCommit: (id: string, start: number, end: number, layer: number) => void, onSel: (id: string) => void,
): { drag: Drag | null; beginDrag: BeginDrag; rows: number }
```

### Inputs

- `regions` - the lane's already row-assigned regions (`layers.ts`'s `layoutRegions`, or a lane hook built on it such as `useLayoutLaneRegions` / `useCaptionLaneRegions`). Pass the MEMOIZED array: `useRegionDrag` rebuilds `beginDrag` whenever this identity changes, and a fresh array every render would defeat `RegionRows`' `React.memo` as well.
- `rowHeightPx` - an argument, not a constant, because the row height comes from the density ladder (`shell/density.ts` via `useDensity`) and changes with the editor's density setting. This is the one place the extraction differs from the plan's sketch, which predates density.
- `onSel` is typed `(id: string) => void` to match `useRegionDrag`; `Timeline.tsx`'s own `(id: string | null) => void` is assignable to it.

### Returns

`drag` and `beginDrag` straight from `useRegionDrag` (`Drag` and `BeginDrag` are that hook's own exported types), plus `rows`: the number of stacked rows this lane needs right now.

`rows` is `0` for an empty lane, and that zero is load-bearing: `Timeline.tsx` reads it as "do not render this lane at all", so an empty lane shows no gutter label and takes no vertical space. Only the Camera and Audio lanes, which are always present, skip that test.

### Used by

`Timeline.tsx` - one call each for the zoom, FX, captions and layout lanes.
