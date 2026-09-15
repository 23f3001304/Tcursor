# src/editor/timeline/model/pillGeometry.ts

Region-pill left-offset/width geometry shared by the zoom/fx/layout timeline lanes.

## pillLeftPct

```ts
export function pillLeftPct(startMs: number, durMs: number): number
```

A region pill's left offset, in percent of the track: the raw `start/dur` position, except a region starting at (or before) the clip's own beginning (`startMs <= 0`) is nudged in by `PILL_EDGE_INSET_PCT` instead of sitting exactly on the track's true left edge - see `PILL_EDGE_INSET_PCT`'s own doc for why. Pills that don't start at the true edge are unaffected. Returns `0` for `durMs <= 0` rather than dividing by zero.

### Used by

`RegionRows` (`../lanes/RegionRows.tsx`) - the pill's `left` inline style, and as the default `leftPct` argument to `pillWidthPct` below.

## pillWidthPct

```ts
export function pillWidthPct(startMs: number, endMs: number, durMs: number, leftPct = pillLeftPct(startMs, durMs)): number
```

A region pill's rendered width, in percent of the track: the raw `(end-start)/dur` span, floored at `MIN_PILL_PCT` (2.5 - so short regions stay draggable) and capped so `leftPct + width` never reaches the track's true right edge (`100% - PILL_EDGE_INSET_PCT`).

**Why the cap (Task 11, ux audit #26; extended gate-feedback item 1, user-reported 2026-09-02):** `TrimOverlay`'s in/out handles (`.e-trimhandle`) sit at the SAME x position as the clip's own 0%/100% whenever it's untrimmed - its default state. A region pill whose `start_ms`/`end_ms` equals 0/the clip's own duration used to render flush against that edge (right-only originally; the left edge got no inset at all until this item), painting edge-to-edge with the handle's line and, worse, leaving its own resize handle (`.e-zh`, `RegionRows.tsx`) with no room to be grabbed independently of the trim handle. `pillWidthPct` insets any edge the pill's raw span actually reaches by `PILL_EDGE_INSET_PCT` (0.6%) on that side, leaving a small gap before the handle. Pills that don't reach an edge are unaffected on that side - the cap only bites once `leftPct + rawWidth` would exceed it.

`leftPct` defaults to `pillLeftPct(startMs, durMs)` so the two functions stay geometrically consistent by construction even if a caller only calls `pillWidthPct` directly; pass the exact value explicitly if the caller already computed it separately (as `RegionRows` does, to avoid computing it twice).

Returns `0` for `durMs <= 0` rather than dividing by zero. Unit-tested (`pillGeometry.test.ts`).

### Used by

`RegionRows` (`../lanes/RegionRows.tsx`, shared by the zoom/fx/layout lanes) - the pill's `width` inline style.

## PILL_EDGE_INSET_PCT

```ts
export const PILL_EDGE_INSET_PCT = 0.6;
```

The edge inset `pillLeftPct`/`pillWidthPct` apply on whichever side (or both) a pill's raw span actually touches the track's true 0%/100%, in percent of track width - exported so a caller/test can reference the exact same number rather than re-deriving it. Originally right-edge-only (Task 11); `timeline.css` additionally lifts a HOVERED/SELECTED/DRAGGING pill's own z-index above `TrimOverlay`'s handle so the resize handles stay grabbable even where this percent inset alone isn't enough physical separation at a narrow track width (gate-feedback item 1, fix round 1: gated on interactivity, not on span, so an edge-touching pill that's ALSO independently trimmed away doesn't paint over `TrimOverlay`'s dimming stripe at rest).

## MIN_PILL_PCT

```ts
export const MIN_PILL_PCT = 2.5;
```

The minimum pill width `pillWidthPct` floors at, in percent of track width - unchanged from the inline constant it originally replaced.

## PILL_GAP_PCT

`0.15` - the breathing room left between a floored pill and the next pill in its row, as a percent of the track.

**`pillWidthPct` and its neighbour (2026-09-15).** The optional fifth argument `nextStartMs` is the start of the next region in the same row. The `MIN_PILL_PCT` floor now grows a tiny pill only up to that neighbour minus `PILL_GAP_PCT`, never over it, and a pill is never made narrower than its real width. Before this, every one of an 87-caption take's pills sat at the 2.5% floor and adjacent captions overlapped in pixels even though their times did not, which is why the captions lane read as a wall. `RegionRows` sorts each row by start and passes the neighbour; a row's last pill keeps the plain floor.
