# src/editor/timeline/useRangeSelect.ts

The ruler's Shift+drag: the one way to say "this stretch" before pressing Cut or Speed 2x. Mounted by `Ruler` (`./timelineRuler.tsx`); the value it produces lives in `Editor.tsx` and travels through `SlotProps` as `range`/`setRange`, because two different areas need it - the timeline draws it (`RangeOverlay`) and the transport acts on it (`TransportTools`).

## Range

```ts
export type Range = [number, number];
```

An ordered stretch of **clip** ms (low, high) - the same clock every pill on the timeline sits on, and the clock the `add_cut` / `set_speed` ops take. `null` everywhere means "no range", which is what Cut and Speed fall back from (see `TransportTools.md`'s `actionSpan`).

## RangePointer

```ts
export interface RangePointer { clientX: number; shiftKey: boolean; preventDefault(): void; stopPropagation(): void }
```

The subset of a `React.PointerEvent` the gesture reads, kept structural (the way `hooks/keymap.ts`' `KeyLike` is) so the hook can be driven from a plain object in a test rather than through a synthesized DOM event.

## rangeOf

```ts
export function rangeOf(ax: number, bx: number, rect: { left: number; width: number }, dur: number): Range
```

The ordered clip-ms pair two x positions describe against `rect`, each clamped into `[0, dur]` and rounded. Ordering happens after the mapping, so dragging right-to-left produces the same range as left-to-right. Pure, and pinned directly by `useRangeSelect.test.ts`.

## useRangeSelect

```ts
export function useRangeSelect({ dur, trackRef, range, setRange }: {
  dur: number;
  trackRef: RefObject<HTMLDivElement | null>;
  range: Range | null;
  setRange: (r: Range | null) => void;
}): (e: RangePointer) => boolean
```

Returns the `pointerdown` handler `Ruler` offers every press to. It answers **whether it claimed the press**: `true` for a Shift+press (which it `preventDefault`s and `stopPropagation`s), `false` for anything else, so a plain drag falls through to the ruler's own scrub untouched. `dur <= 0` declines too - there is no clip to select inside yet.

### Behavior

**One value, not two.** The live range is written straight into `setRange` as the drag moves, rather than held as a local draft that is committed on release. That means the overlay and the transport can never disagree about what is selected, at the cost of a render per update - which is why every update goes through `useRafCoalesced`: a pointermove burst costs at most one render per animation frame, and `pointerup` schedules the release position and then `flush`es it so the final value lands without waiting for a frame.

**Threshold.** The first write is gated on `pastDragThreshold` (`../hooks/dragThreshold.ts`, 3px), and a press clears the previous range immediately - so a bare Shift+**click** clears the selection rather than leaving a zero-width range behind that Cut would then refuse to act on.

**Escape clears.** A window `keydown` listener drops the range, bound only while there IS one - so a surface that owns Escape for its own dismiss (stage arrange mode) keeps it whenever nothing is selected here. This is deliberately separate from `keymap.ts`' `"deselect"` action, which owns the *region* selection (`sel`): the two are independent axes and either can be live without the other.

**Listener lifecycle.** The two window listeners are keyed on drag PRESENCE (a boolean `useState`), so they attach once per drag and tear down once per drag rather than once per pointermove - the same fix `useRegionDrag.md` records for its own listeners. `dur` and `setRange` are read from refs inside them, so neither listener depends on a value that changes every render.

### Notes

- The ms mapping uses `trackRef` (`.e-tlbody`), not the ruler's own box - see `timelineRuler.md` for why the two are interchangeable and why that matters.
- Nothing here touches the doc. A range is UI state; it becomes an edit only when `TransportTools` applies `add_cut` or `set_speed` with it, and is cleared by that action.
