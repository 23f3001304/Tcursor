# src/editor/hooks/input/useRegionDrag.ts

Shared drag/resize logic for timeline region pills (zoom + effects + layout segments). Body = move
(both horizontally to retime, and vertically to change priority layer), side handles = resize; a
drag updates a local draft (`drag`) and calls `onCommit` on release.

## Mode

```ts
export type Mode = "move" | "l" | "r"
```

Which part of a pill is being dragged: whole body, left handle, or right handle.

## Drag

```ts
export interface Drag { id: string; mode: Mode; oStart: number; oEnd: number; start: number; end: number; lo: number; hi: number; layer: number; dyPx: number }
```

The in-flight drag: the region `id`, the `mode`, the original span (`oStart`/`oEnd`), the live
draft span (`start`/`end`), the clamp bounds (`lo`/`hi` = the same-layer neighbour gap, frozen at
`beginDrag`), the live snapped layer (`layer`, move only), and the raw unsnapped vertical pixel
offset since the drag began (`dyPx` - the caller applies it as a live `translateY` so the dragged
pill follows the pointer smoothly without remounting into a different row's DOM mid-drag).

## BeginDrag

```ts
export type BeginDrag = (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => void
```

Exactly the signature `useRegionDrag` already returned as `beginDrag`, given a name in M5 T4 so the wrapper that now sits between it and `Timeline.tsx` (`timeline/useLaneDrag.ts`) can write its own return type down instead of re-deriving this by hand or widening it to `any`. Nothing about the hook changed; this is a name for a shape that was always there.

## useRegionDrag

```ts
export function useRegionDrag(
  regions: { id: string; start_ms: number; end_ms: number; layer?: number }[],
  dur: number, trackRef: React.RefObject<HTMLDivElement | null>, rowHeightPx: number,
  onCommit: (id: string, start: number, end: number, layer: number) => void, onSel: (id: string) => void,
): { drag: Drag | null; beginDrag: (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => void }
```

### Behavior

- `beginDrag` selects the region and computes `lo`/`hi` from same-layer neighbours (`layer ?? 0`), then starts the draft. Memoized (`useCallback`, deps `[regions, dur, onSel]`) so its identity only changes when the region set/duration/selection-handler actually change - not on every render - letting a `React.memo`'d pill component's `onPointerDown={beginDrag}` prop stay stable. It also records the pointer's starting `clientX`/`clientY` (into refs, for the threshold check below).
- While `drag` is non-null, window `pointermove`/`pointerup` listeners update the draft - move keeps the width inside `[lo, hi]` and snaps a vertical drag to a `layer` (dragging up raises priority); a handle keeps a 150ms minimum span and never changes layer.
- **`onCommit` only fires past a drag threshold (bug-sweep-2 Task 8, M8).** `up` checks `pastDragThreshold(e.clientX - startX, e.clientY - startY)` (`../../util/dragThreshold.ts`, default 3px) against the pointer's position at RELEASE vs. where `beginDrag` started - only past that does it call `onCommit(id, round(start), round(end), layer)`; otherwise the draft is just discarded. `onSel` already ran unconditionally in `beginDrag`, so a bare click still selects the region - it just no longer ALSO commits a no-op `update_zoom`/`update_effect`/`update_layout_seg` (an undo step + a full IPC round trip for a span that never actually changed - UX audit #5: clicking a spotlight pill nudged its Start `0s -> 0.03s`).
- **Listeners attach once per drag, not once per pointermove.** The attach/detach effect is keyed on drag PRESENCE alone (`drag !== null`, a boolean) rather than on the `Drag` object itself (which `setDrag` replaces on every move) or on `dur`/`onCommit`/`rowHeightPx` (a fresh inline arrow/value from the caller most renders) - those three are read from refs updated every render instead, so the effect's own dependency array never sees a change mid-drag and the two `window` listeners are added exactly once at drag-start and removed exactly once at drag-end. Before this, the effect re-ran (tearing down and re-adding both listeners) on every pointermove, times however many `useRegionDrag` instances were mounted (Timeline mounts three: zoom/effects/layout).
- Multiple instances (zoom + effects + layout) never cross-talk - each owns its own `drag` state and its own pair of listeners.
