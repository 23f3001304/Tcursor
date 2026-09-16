# src/editor/timeline/laneBuilders.tsx

The shared building blocks `useTimelineLanes.tsx` assembles its `TimelineLane[]` from: the lane-height arithmetic, the zoom/FX gutter labels, and `regionLane` - the one drag-and-pill lane builder the zoom, FX, captions and layout lanes all go through. Split out of `useTimelineLanes.tsx` (editor-parity foundations, task 9) so a new region lane has a stable, non-closure helper to call: Batch 2c's text lane and Batch 4's clip lane each add their call to `regionLane` (or push a lane directly) from `useTimelineLanes.tsx`, not here.

## TimelineLane

```ts
export interface TimelineLane {
  key: string;
  label: string;
  heightPx: number;
  active: boolean;
  body: React.ReactNode;
}
```

One visible lane: the key both columns hang their `e-lane-${key}` class off, the gutter label, the height that label slot has to match, whether this lane owns the current selection, and the rows themselves. Re-exported from `useTimelineLanes.tsx` (`export type { TimelineLane } from "./laneBuilders";`) so `Timeline.tsx`'s import path didn't have to change.

## LaneCtx

```ts
export interface LaneCtx {
  ROW_H: number;
  GAP: number;
  dur: number;
  sel: string | null;
  isSel: (regions: { id: string }[]) => boolean;
}
```

What `regionLane` used to close over as a hook-body function, now passed explicitly: the row height and gap `useDensity()` resolved, the timeline's total duration, the current selection id, and the lane-active predicate. `useTimelineLanes.tsx` builds one `cx` object per render, right after its own `isSel` is defined, and passes that same object to all four `regionLane` calls.

## laneHeight

```ts
export const laneHeight = (rows: number, rowH: number, gap: number): number
```

`rows > 0 ? rows * rowH + (rows - 1) * gap : 0` - the pixel height of a stack of `rows` pill rows, `gap` apart, or `0` for an empty lane (so it renders no gutter slot at all). The one function both the label gutter and the row bodies size themselves from, so a lane's gutter label can never drift out of step with its actual row height. Used by `regionLane` below, and directly by `useTimelineLanes.tsx` for the Audio lane, which has no `regionLane` call of its own.

## zoomLabel

```ts
export const zoomLabel = (z: { scale: number }): React.ReactNode
```

The Zoom lane's pill label: an `IconZoomIn` plus the region's scale to one decimal (e.g. `1.5x`). Passed to `regionLane` as `renderLabel`.

## fxLabel

```ts
export const fxLabel = (e: { kind: string }): React.ReactNode
```

The FX lane's pill label: the region's own icon plus its kind's name, looked up in the module's `FX_KINDS` table - `IconBulb` Spotlight, `IconBlur` Blur, `IconGridDots` Pixelate, `IconFocus2` Highlight. Passed to `regionLane` as `renderLabel`, whose signature already supplies the region, so no call site changed when it stopped ignoring its argument.

*Why the icon carries the kind and not the colour:* all four kinds share the `--e-fx` accent and the `.e-fxblk` pill, because they are one lane. Four accents on one lane would read as four lanes. An unknown kind falls back to the spotlight entry rather than rendering an empty pill.

Both `zoomLabel` and `fxLabel` are MODULE-scope, not declared inline in a component body - they're pure and close over nothing, so a fresh inline arrow every render would have been a fresh `renderLabel` prop every render, defeating `RegionRows`' `React.memo` even when the underlying region data hadn't changed.

## regionLane

```ts
export function regionLane<T extends Region>(
  lanes: TimelineLane[],
  cx: LaneCtx,
  key: string,
  label: string,
  lane: { rows: number; drag: Drag | null; beginDrag: BeginDrag },
  regions: T[],
  rowClass: string,
  blkClass: string,
  renderLabel: (r: T) => React.ReactNode,
  extraStyle?: (r: T, s: number, e: number) => React.CSSProperties,
  titleOf?: (r: T) => string | undefined,
): void
```

Pushes one lane onto `lanes`, or nothing at all when `lane.rows === 0` (an empty zoom/FX/captions/layout lane renders no gutter row). The one helper the Zoom, FX, Captions and Layout lanes all go through - they differ only in their key, label, row/pill classes, and label render-prop. The Time, Camera and Audio lanes each render a component of their own and are pushed onto `lanes` directly by `useTimelineLanes.tsx` instead.

### Implementation

Builds one `TimelineLane` - `heightPx` from `laneHeight(lane.rows, cx.ROW_H, cx.GAP)`, `active` from `cx.isSel(regions)` - whose `body` is a `<RegionRows>` (`./lanes/RegionRows.tsx`) carrying `regions`, `cx.dur`, `cx.sel`, the two class names, `lane.drag`/`lane.beginDrag` (from that lane's own `useLaneDrag` call, made by the caller before `regionLane` runs), and the three render-props, then pushes it.

### Notes

- Generic over `T extends Region` so one function serves regions shaped like a `Zoom`, an `EffectRegion`, a `Caption`, or a `LayoutSeg` - `regionLane` only ever reads `id`/`start_ms`/`end_ms`/`layer` through `Region`, `RegionRows`, and `cx.isSel`. `Region` itself is not exported; nothing outside this file needs to name it.
- Takes `lanes` and `cx` as explicit leading arguments rather than closing over them, so it is a plain module-scope function instead of a per-render closure recreated on every `useTimelineLanes` call. Its identity is never passed to a memoized child either way (see `useTimelineLanes.md`'s Render hygiene section), so this is about not repeating five free variables across four call sites, not about render performance.

### Used by

`src/editor/timeline/useTimelineLanes.tsx` - four calls (zoom, FX, captions, layout), each with its own key/label/classes/render-props but the same `lanes`/`cx`.
