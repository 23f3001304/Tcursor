# src/editor/timeline/RegionRows.tsx

Shared row/pill renderer for the zoom, FX, and layout lanes in `Timeline.tsx` (Task 36). The three lanes are identical in every way except fill color (a CSS class), the pill's label content, and (layout only) its inline fade-ramp CSS vars - this component takes those as props instead of the three lanes triplicating the same ~25 lines of `motion.div`/grip-handle markup each. Extracted so `Timeline.tsx` stays under its line budget even with the lane-label gutter (see `Timeline.md`'s "Lane label gutter" / "Why a sibling gutter" notes).

## Region

```ts
interface Region { id: string; start_ms: number; end_ms: number; layer: number }
```

The minimal shape `RegionRows` needs from a region - satisfied by the `layoutRegions`-assigned zoom/effect/layout-segment arrays `Timeline.tsx` passes in.

## RegionRows

```tsx
export function RegionRows<T extends Region>({ rows, regions, dur, sel, rowClass, blkClass, dragState, beginDrag, renderLabel, extraStyle }: {
  rows: number; regions: T[]; dur: number; sel: string | null;
  rowClass: string; blkClass: string;
  dragState: Drag | null;
  beginDrag: (ev: React.PointerEvent, id: string, mode: "move" | "l" | "r", s: number, e: number) => void;
  renderLabel: (r: T) => React.ReactNode;
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties;
}): JSX.Element
```

### Props

- `rows: number` - row count for this lane (`maxLayer + 1`, `0` if empty - `Timeline.tsx` computes this the same way it always did and only renders `RegionRows` when `rows > 0`).
- `regions: T[]` - the lane's `layoutRegions`-assigned regions.
- `dur: number` / `sel: string | null` - clip duration (for the `%` position math) and the currently-selected region id.
- `rowClass: string` / `blkClass: string` - the row and pill CSS classes (`"e-zoomrow"`/`"e-zblk"`, `"e-fxrow"`/`"e-fxblk"`, or `"e-layrow"`/`"e-layblk"`) - the only thing distinguishing the three lanes visually.
- `dragState: Drag | null` - the lane's own `useRegionDrag().drag` value; while a region in THIS lane is dragging, its live `start`/`end`/`dyPx` override the region's own `start_ms`/`end_ms`/render position.
- `beginDrag` - the lane's own `useRegionDrag().beginDrag`, wired to the pill body (`"move"`) and both edge handles (`"l"`/`"r"`).
- `renderLabel: (r: T) => React.ReactNode` - the pill's icon + text (zoom: `IconZoomIn` + `{scale}x`; FX: static `IconBulb` + "Spotlight"; layout: `IconAspectRatio` + `prettyLayout(layout)`).
- `extraStyle?: (r, s, e) => React.CSSProperties` - additional inline style merged onto the pill (layout only: the `--fin`/`--fout` fade-ramp CSS vars from `transitionRampPct`).

### Behavior

For each layer `0..rows-1` (rendered top-to-bottom as `rows-1..0`, matching the original per-lane loops so higher layers still paint above lower ones), renders one `<div className={rowClass}>` containing every region assigned that layer. Each pill is a `motion.div` with `data-region-id={r.id}` (harmless on layout pills - nothing currently queries it there, only zoom/FX ids are looked up by `src/editor/director/targets.ts`), positioned `left: (s/dur)*100%`, `width: max(2.5%, ((e-s)/dur)*100%)`, fading/scaling in on mount and hover (`whileHover: scale 1.02`), with `y` following `dragState.dyPx` at zero-duration while dragging (instant, no spring lag) so the pill visually tracks the pointer without remounting into a different row's DOM mid-drag - it only actually reflows into its new row once the drag ends and `regions` re-renders with the committed layer. Body `pointerDown` starts a `"move"` drag; the two `.e-zh` side handles start `"l"`/`"r"` resizes.

### Used by

`Timeline.tsx` - once each for the zoom, FX, and layout lanes, as the `body` of their `lanes` array entry (rendered inside `.e-tracks`, with the matching label rendered separately into `.e-lanegutter` - see `Timeline.md`).
