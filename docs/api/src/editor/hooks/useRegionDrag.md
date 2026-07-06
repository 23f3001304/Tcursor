# src/editor/hooks/useRegionDrag.ts

Shared drag/resize logic for timeline region pills (zoom + effects). Body = move, side handles = retime; a drag updates a local draft and calls `onCommit` on release. The draft is clamped to the gap between same-layer neighbours, so regions on one layer can't overlap (overlapping effects live on different layers).

## Mode

```ts
export type Mode = "move" | "l" | "r"
```

Which part of a pill is being dragged: whole body, left handle, or right handle.

## Drag

```ts
export interface Drag { id: string; mode: Mode; oStart: number; oEnd: number; start: number; end: number; lo: number; hi: number }
```

The in-flight drag: the region `id`, the `mode`, the original span (`oStart`/`oEnd`), the live draft span (`start`/`end`), and the clamp bounds (`lo`/`hi` = the same-layer neighbour gap, frozen at `beginDrag`).

## useRegionDrag

```ts
export function useRegionDrag(
  regions: { id: string; start_ms: number; end_ms: number; layer?: number }[],
  dur: number, trackRef: React.RefObject<HTMLDivElement | null>,
  onCommit: (id: string, start: number, end: number) => void, onSel: (id: string) => void,
): { drag: Drag | null; beginDrag: (e: React.PointerEvent, id: string, mode: Mode, s: number, en: number) => void }
```

### Behavior

- `beginDrag` selects the region and computes `lo`/`hi` from same-layer neighbours (`layer ?? 0`), then starts the draft.
- While `drag` is non-null, window `pointermove`/`pointerup` listeners update the draft - move keeps the width inside `[lo, hi]`; a handle keeps a 150ms minimum span - and `onCommit(id, round(start), round(end))` fires on release. Listeners attach only during the drag, so multiple instances (zoom + effects) never cross-talk.
