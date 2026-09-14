# src/editor/timeline/timelineRuler.tsx

The timeline's top strip and the scrub math behind it, extracted from `Timeline.tsx` (which sat exactly on its 200-line cap) so the range gesture had somewhere to live. Three exports: the shared x-to-ms mapping (`useSeek`), the tick strip itself with its two gestures (`Ruler`), and the selection's own overlay (`RangeOverlay`).

Both the ruler and the track body measure against the **same** element - `trackRef`, i.e. `.e-tlbody` - rather than each against its own box. `.e-ruler` and `.e-tlbody` are siblings inside `.e-timeline` and therefore the same content width, so a tick, a pill and the playhead at the same ms always land on the same x, and the pills' existing ms/px math is untouched by the ruler becoming interactive.

## useSeek

```ts
export function useSeek(dur: number, trackRef: RefObject<HTMLDivElement | null>, onSeek: (ms: number) => void): {
  seekAt: (clientX: number) => void;
  scheduleSeek: Coalesced<number>;
  flushSeek: () => void;
}
```

`seekAt` maps a pointer x into `[0, dur]` against `trackRef`'s `getBoundingClientRect()` and calls `onSeek` - the immediate jump-to-click. `scheduleSeek` is the same call through `useRafCoalesced` (`../hooks/useRafCoalesced.ts`): a pointermove burst, which on a high-poll-rate mouse outpaces the display refresh rate, collapses to at most one seek per animation frame, always with the latest x. `flushSeek` applies a still-pending one immediately, so the exact release position lands on `pointerup` instead of waiting out another frame.

This is a verbatim move of what `Timeline.tsx` did inline before Task 7; the ruler now calls it as well, so the two surfaces cannot drift into two different mappings.

## Ruler

```tsx
export function Ruler({ dur, trackRef, onSeek, range, setRange }: {
  dur: number; trackRef: RefObject<HTMLDivElement | null>; onSeek: (ms: number) => void;
  range: Range | null; setRange: (r: Range | null) => void;
}): JSX.Element
```

`.e-ruler` with one `<span>` per `rulerTicks(dur)` entry at `(at / dur) * 100%` (unchanged), plus the two gestures that start on it:

- **Shift+drag selects a range.** `useRangeSelect` (`./useRangeSelect.ts`) is offered the `pointerdown` first; when it claims it, nothing else runs. See that file's doc for the gesture itself.
- **A plain drag scrubs.** The ruler captures the pointer, `seekAt`s immediately, then `scheduleSeek`s each move while the primary button is held and `flushSeek`es on release or lost capture - the exact contract `.e-tlbody` has, so the strip above the tracks behaves like the tracks.

`title` names both ("Drag to scrub. Shift+drag to choose a range."), which is the one place the gesture is discoverable outside `ShortcutsOverlay`. `timeline.css` gives `.e-ruler` `cursor: pointer`, `user-select: none` and `touch-action: none` so the shift-drag does not also run the browser's own text selection.

## RangeOverlay

```tsx
export function RangeOverlay({ range, dur }: { range: Range | null; dur: number }): JSX.Element | null
```

One `.e-range` div spanning the selection across the whole track body - rendered by `Timeline.tsx` inside `.e-tlbody`, next to `TrimOverlay`, so the selection reads against the lanes it will act on rather than only against the 18px ruler the gesture started on. A translucent `--e-primary` wash at 12% with a 1px accent edge at each end (timeline.css), no border and no handles: the gesture that made it is how it is changed, and Escape is how it goes away. `pointer-events: none`, so scrubbing and pill drags pass straight through it. `null` when there is no range, when `dur <= 0`, or when the range is empty.

Static CSS, no Motion - nothing here animates.

### Notes

- The overlay draws `range`, not a separate live draft: `useRangeSelect` writes the live value into `setRange` directly (rAF-coalesced), so what is drawn and what `TransportTools` would act on are the same value by construction.
