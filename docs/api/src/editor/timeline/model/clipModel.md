# src/editor/timeline/model/clipModel.ts

The Clips lane's own model: `doc.clips` turned into the pills `ClipLane.tsx` draws, the pill's label, its fade ramp, and the arithmetic a drag needs to name a drop target.

**The axis ruling.** The timeline has ONE axis, the recording's own clock, and every other lane, the ruler, the playhead, the trim overlay and the cut overlay are on it. The Clips lane stays on it: a clip's pill spans its SOURCE range, `src_in_ms` to `src_out_ms`, so a pill never lies about where its material sits in the recording. Output ORDER is carried by the pill's label ("1", "2", "3") and by the inspector, not by the pill's x position. Dragging a pill's body therefore means "put this clip where that one is": on release the clip under the pointer names the target index and `move_clip` runs, the pill returns to its source range, and the numbers change. Dragging an edge retimes `src_in`/`src_out` through `update_clip`. This is the one lane whose pills can overlap (two clips may cover the same source range after an edge drag), which is why `layoutRegions` assigns the rows.

**The badge ruling.** The transition badge sits on the LEFT edge, not the right: `transition_in_ms` dissolves INTO this clip, and with pills on the source axis the outgoing clip may be anywhere, so the incoming clip's own left edge is the only honest place for it. It reuses the `--fin` ramp `layoutExtraStyle` (`LayoutLane.tsx`) already drives.

## ClipRegion

```ts
export interface ClipRegion {
  id: string; start_ms: number; end_ms: number; layer: number;
  order: number; outMs: number; transition_in_ms: number;
}
```

A clip laid out on the timeline. `start_ms`/`end_ms` are the SOURCE range (`src_in_ms`/`src_out_ms`), so `RegionRows`/`pillGeometry.ts` can position it exactly like any other region. `layer` is `layoutRegions`' row assignment. `order` is this clip's 1-based position in `doc.clips` - the export order. `outMs` is how long it plays in the OUTPUT, not the source. `transition_in_ms` rides along so `clipExtraStyle` doesn't need the raw `Clip` too.

## clipRegions

```ts
export function clipRegions(clips: Clip[], map: TimeMap): ClipRegion[]
```

`layoutRegions` (`./layers.ts`) gives the rows, the same greedy interval-partitioning every other region lane uses - the Clips lane is the one lane that actually needs it for genuine OVERLAP rather than for tidiness, since two clips can cover the same source span once an edge has been dragged. `order` is read off `clips`' own array position (index + 1) BEFORE `layoutRegions` re-sorts its output by `start_ms`, so the label still says where a clip sits in the OUTPUT even once its pill draws earlier or later on the source axis than its neighbours. `outMs` is `clipOutMs(map, i)` (`../../../shared/math/remap.ts`) at that same index, so the label says what the export will actually spend on this clip - cuts and speed changes folded in - not the raw `src_out_ms - src_in_ms` span the pill's width already shows. An empty `clips` returns `[]`, but that is not what gates the lane: `useTimelineLanes.tsx` reads `doc.clips.length > 1` before it calls this at all, so a document with ONE clip hides the Clips lane too, and `clipRegions` is never the thing that decides.

## clipLabel

```ts
export const clipLabel: (c: ClipRegion) => string
```

`` `${order} - ${(outMs / 1000).toFixed(1)}s` `` - the pill's whole label. Order first, because that is what a click needs to find a clip by; the output length second, because that is what a reorder drag needs to weigh.

## clipExtraStyle

```ts
export const clipExtraStyle: (c: ClipRegion, s: number, e: number) => React.CSSProperties
```

`{ "--fin": transitionRampPct(transition_in_ms, e - s) + "%" }` - the same `--fin` custom property `layoutExtraStyle` sets for the Layout lane, read here by `.e-clipblk::before` (`timeline.css`) as a left-edge gradient instead of `.e-layblk`'s two-sided one. A hard cut (`transition_in_ms === 0`) is `0%` and therefore invisible, exactly as an untransitioned layout segment is.

## dropIndex

```ts
export function dropIndex(clips: Clip[], draggedId: string, atMs: number): number | null
```

The clip under the pointer names the target index: the first clip OTHER than `draggedId` whose source range contains `atMs`, by position in `clips` itself (not in the `layoutRegions`-sorted output). `move_clip` removes the dragged clip from the list and then re-inserts it at that index, which is the standard list-move semantics - dropping a later clip onto an earlier one puts it BEFORE that clip, dropping an earlier one onto a later one puts it AFTER it - and is exactly what the pill's `title` (`ClipLane.tsx`) promises. Returns `null` when no other clip's source range covers `atMs` (empty space, or the point sits only inside the dragged clip's own span) and also when `draggedId` is not actually a clip in the list, so a stray or stale drag commits nothing rather than moving the wrong thing.

### Used by

- `src/editor/timeline/lanes/ClipLane.tsx` - `clipRegions`/`clipLabel`/`clipExtraStyle` build the lane's pills, `dropIndex` resolves a body drag's drop target.
- `src/editor/timeline/useTimelineLanes.tsx` - `clipRegions` a second time, to size the lane's row count before `ClipLane` itself mounts.
