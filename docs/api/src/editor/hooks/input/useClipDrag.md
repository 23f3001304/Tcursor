# src/editor/hooks/input/useClipDrag.ts

The Clips lane's own drag/resize hook. Same shape as `useRegionDrag.ts` (a `drag` draft plus a `beginDrag` starter, one window `pointermove`/`pointerup` pair attached only while a drag is live), but not a wrapper around it - three things about a clip pill make the commit and the clamping genuinely different from every other region lane.

## MIN_CLIP_MS

```ts
export const MIN_CLIP_MS = 100;
```

How narrow an edge drag may make a clip, in source milliseconds (`useRegionDrag`'s handles keep 150ms; a clip is more often trimmed to a short beat than a zoom region is, so the floor is tighter). Exported rather than kept inline because the Clip inspector's `TimingRow` needs the SAME floor: its Start stepper was bounded only by End, and `clipops::update_clip` removes a clip whose bounds meet, including the last one, which reverts the document to the whole trim range past `remove_clip`'s own guard. One constant, two surfaces, so a change to the floor can never move one of them and leave the other behind.

## useClipDrag

```ts
export function useClipDrag(
  regions: { id: string; start_ms: number; end_ms: number; layer: number }[],
  dur: number, trackRef: React.RefObject<HTMLDivElement | null>,
  onReorder: (id: string, atMs: number) => void,
  onRetime: (id: string, srcIn: number, srcOut: number) => void,
  onSel: (id: string) => void,
): { drag: Drag | null; beginDrag: (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => void }
```

Reuses `useRegionDrag`'s own `Drag`/`Mode` types (`./useRegionDrag.ts`) - the draft shape is identical, only what happens on release differs - and `pastDragThreshold` (`../../util/dragThreshold.ts`) for the same past-3px-or-it-was-a-click rule every other region drag honours.

### Why not `useRegionDrag`

- **A body drag commits an INDEX, not a time.** Every other region lane's body drag ends in `onCommit(id, start, end, layer)` - a new span. A clip's body drag ends in `onReorder(id, atMs)`: the midpoint of wherever the pill was dropped, which `ClipLane.tsx` turns into a target index with `dropIndex` and commits as `move_clip`. The pill's own `start`/`end` are never part of that commit - per the axis ruling (`clipModel.md`), a clip's source range does not change just because it changed EXPORT position.
- **There is no layer drag.** `useRegionDrag` snaps a vertical drag to a row so a pill can change priority layer; a clip's `layer` is display-only (`layoutRegions`' overlap bookkeeping, exactly like `TimeLane`'s speed spans), so `beginDrag` records the region's current layer once, into the draft, and nothing in `move`/`up` ever recomputes it.
- **Neighbours never clamp an edge drag.** `useRegionDrag` bounds a resize against the nearest same-layer neighbour (`lo`/`hi`, frozen at `beginDrag`) so two zoom regions can't be dragged into overlapping. A clip pill is allowed to overlap another - that's the whole reason `clipRegions` runs `layoutRegions` for real row assignment rather than a single always-zero row - so `lo`/`hi` here are simply `[0, dur]`, the clip's own duration bounds, and a `move` drag clamps only against those, never against a sibling clip's span.

### Behavior

- `beginDrag` selects the clip and starts the draft exactly as `useRegionDrag` does, with `lo: 0, hi: dur` in place of a neighbour-derived gap.
- While dragging, `move` keeps a body drag inside `[0, dur]` and an edge drag at least `MIN_CLIP_MS` wide.
- `up`, past the drag threshold, calls `onReorder` for a `"move"` drag (the draft's midpoint, rounded) or `onRetime` for an edge drag (the draft's `start`/`end`, rounded) - never both, and never a `layer`.

### Behaviors

Driven end to end through `ClipLane.tsx` (`ClipLane.test.tsx`, Batch 4 T8 fix round 1 - `dispatchEvent`/`act`, no testing library, the same idiom `TimeLane.test.tsx` already drives `useRegionDrag` with), since `onReorder`/`onRetime` only become the actual `EditOp` the lane sends once `ClipLane`'s own callbacks run.

- `dragging a pill's body onto an earlier clip's position moves it to the front` / `...onto a later clip's position moves it to the back` - covered from both directions, because `move_clip`'s remove-then-insert is not symmetric: `dropIndex`'s target is the OTHER clip's own pre-removal index either way, which lands before it when a later clip moves earlier and after it when an earlier clip moves later.
- `dragging the left edge commits update_clip with the new src_in_ms and the untouched src_out_ms` / `dragging the right edge past the 100ms floor clamps instead of crossing the left edge` - an edge drag always carries BOTH bounds (the untouched one at its current value, the exact contract `TimeLane`'s `update_speed` commit already uses - its own test pins `start_ms: 1000` unchanged beside a dragged `end_ms`), and the dragged one is rounded to a whole millisecond and clamped to the 100ms floor before it ever reaches `onRetime`.
- `a press and release under the drag threshold selects but sends no op` - `onSel` runs unconditionally in `beginDrag`, before any threshold check, so a bare click still selects; `up` never calls `onReorder`/`onRetime` when the release never cleared `pastDragThreshold`.
- `a body drag past the threshold that lands over no other clip sends no op` - `dropIndex` returns `null` when the release point isn't over another clip, and `ClipLane`'s `onReorder` only calls `onApply` when it isn't.
- `an edge drag past the threshold that lands back on the original bound sends no op` - fixed in this round: `ClipLane`'s `onRetime` did not used to check this, and sent `update_clip` with the clip's own current `src_in_ms`/`src_out_ms` whenever a drag went far enough in raw pixels to clear the threshold but clamped or rounded back to where it started (e.g. an edge dragged out and back to the same spot before release). It now looks up the clip by `id` in its own `clips` prop and returns without calling `onApply` when both bounds already match - the same "don't commit a no-op edit" shape `onReorder`'s `to !== null` check already had for the move case.

### Used by

`src/editor/timeline/lanes/ClipLane.tsx` - the lane's only drag source; `onReorder` and `onRetime` there turn into `move_clip` and `update_clip`.
