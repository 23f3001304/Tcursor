# src/editor/timeline/lanes/RegionRows.tsx

Shared row/pill renderer for the zoom, FX, and layout lanes in `Timeline.tsx` (Task 36). The three lanes are identical in every way except fill color (a CSS class), the pill's label content, and (layout only) its inline fade-ramp CSS vars - this component takes those as props instead of the three lanes triplicating the same ~25 lines of `motion.div`/grip-handle markup each. Extracted so `Timeline.tsx` stays under its line budget even with the lane-label gutter (see `Timeline.md`'s "Lane label gutter" / "Why a sibling gutter" notes).

## Region

```ts
interface Region { id: string; start_ms: number; end_ms: number; layer: number }
```

The minimal shape `RegionRows` needs from a region - satisfied by the `layoutRegions`-assigned zoom/effect/layout-segment arrays `Timeline.tsx` passes in.

## RegionRows

```tsx
function RegionRowsInner<T extends Region>({ rows, regions, dur, sel, rowClass, blkClass, dragState, beginDrag, renderLabel, extraStyle }: {
  rows: number; regions: T[]; dur: number; sel: string | null;
  rowClass: string; blkClass: string;
  dragState: Drag | null;
  beginDrag: (ev: React.PointerEvent, id: string, mode: "move" | "l" | "r", s: number, e: number) => void;
  renderLabel: (r: T) => React.ReactNode;
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties;
  titleOf?: (r: T) => string | undefined;
}): JSX.Element

export const RegionRows = memo(RegionRowsInner) as typeof RegionRowsInner;
```

`React.memo`'d (cast back to `RegionRowsInner`'s own generic signature, since `memo` doesn't preserve a generic function's type parameters on its own) - only re-renders when its own props change. This only actually SKIPS work when the caller's props are themselves stable: `Timeline.tsx` passes `useMemo`'d `regions` (keyed on the underlying `doc.*` array), `dragState`/`beginDrag` from `useRegionDrag` (which only changes across a real drag start/end, not per pointermove), and module-level (not inline-per-render) `renderLabel`/`extraStyle` functions.

### Props

- `rows: number` - row count for this lane (`maxLayer + 1`, `0` if empty - `Timeline.tsx` computes this the same way it always did and only renders `RegionRows` when `rows > 0`).
- `regions: T[]` - the lane's `layoutRegions`-assigned regions.
- `dur: number` / `sel: string | null` - clip duration (for the `%` position math) and the currently-selected region id.
- `rowClass: string` / `blkClass: string` - the row and pill CSS classes (`"e-zoomrow"`/`"e-zblk"`, `"e-fxrow"`/`"e-fxblk"`, or `"e-layrow"`/`"e-layblk"`) - the only thing distinguishing the three lanes visually.
- `dragState: Drag | null` - the lane's own `useRegionDrag().drag` value; while a region in THIS lane is dragging, its live `start`/`end`/`dyPx` override the region's own `start_ms`/`end_ms`/render position.
- `beginDrag` - the lane's own `useRegionDrag().beginDrag`, wired to the pill body (`"move"`) and both edge handles (`"l"`/`"r"`).
- `renderLabel: (r: T) => React.ReactNode` - the pill's icon + text (zoom: `IconZoomIn` + `{scale}x`; FX: static `IconBulb` + "Spotlight"; layout, T34 L4: a small `LayoutThumb` schematic of the segment's own resolved panels + "Custom" or `prettyLayout(layout)` - see `LayoutLane.md`).
- `extraStyle?: (r, s, e) => React.CSSProperties` - additional inline style merged onto the pill (layout only: the `--fin`/`--fout` fade-ramp CSS vars from `transitionRampPct`).
- `titleOf?: (r: T) => string | undefined` - a native `title` for the PILL itself (captions only: `captionTitle`, the full line). It is on the pill rather than on the label because the label is `pointer-events: none`, and it exists because the captions lane is the one lane whose pills are routinely too narrow to draw a label at all - see `CaptionLane.md`. Omitted, a pill gets no `title`, exactly as before; returning `undefined` for one region omits it for that region.

### Behavior

For each layer `0..rows-1` (rendered top-to-bottom as `rows-1..0`, matching the original per-lane loops so higher layers still paint above lower ones), renders one `<div className={rowClass}>` containing every region assigned that layer. Each pill is a `motion.div` with `data-region-id={r.id}` (harmless on layout pills - nothing currently queries it there, only zoom/FX ids are looked up by `src/editor/director/targets.ts`), positioned `left: pillLeftPct(s, dur)%`, `width: pillWidthPct(s, e, dur, leftPct)%` (`../model/pillGeometry.ts` - Task 11, ux audit #26, extended gate-feedback item 1 (2026-09-02): both insets exist so a pill whose `start_ms`/`end_ms` lands exactly on the clip's own 0/duration never paints flush against `TrimOverlay`'s in/out handle, see `pillGeometry.md`), fading in on mount (no hover growth: it made a short pill harder to grab and read as a jump, owner ruling 2026-09-13; hover is the CSS lightness step), with `y` following `dragState.dyPx` at zero-duration while dragging (instant, no spring lag) so the pill visually tracks the pointer without remounting into a different row's DOM mid-drag - it only actually reflows into its new row once the drag ends and `regions` re-renders with the committed layer. Body `pointerDown` starts a `"move"` drag; the two `.e-zh` side handles (11px, inward-facing - the pill's own first/last flex children) start `"l"`/`"r"` resizes.

The z-index priority a pill's resize handles need over `TrimOverlay`'s `.e-trimhandle` (z-index 6, timeline.css - the thing an edge-touching pill can otherwise coincide with at that same x position when the clip is untrimmed) lives entirely in `timeline.css` now, NOT here (fix round 1, controller ruling 2026-09-02): `.e-zblk:hover`/`.sel`/`.drag` (and the `.e-fxblk`/`.e-layblk` equivalents) get `z-index: 7` - gated on interactive relevance (hover/selected/dragging), not on whether the pill's raw span touches the clip's edge. The original static "any edge-touching pill, always" version (an inline `zIndex` computed here) painted a pill over `.e-trimdim`'s dimming stripe (z-index 3) whenever that same edge was ALSO independently trimmed - a state-honesty regression. At rest every pill now stays at the CSS-default z-index (stack level 0, below `.e-trimdim`), same as before item 1 existed at all.

### Used by

`Timeline.tsx` - once each for the zoom, FX, and layout lanes, as the `body` of their `lanes` array entry (rendered inside `.e-tracks`, with the matching label rendered separately into `.e-lanegutter` - see `Timeline.md`).

**Neighbour-aware floor (2026-09-15).** Each row is sorted by `start_ms` and `pillWidthPct` receives the next pill's start, so the minimum-width floor stops at the neighbour instead of overlapping it (see `model/pillGeometry.md`).
