# src/editor/timeline/TimeLane.tsx

The Time lane: one pill per speed span, on the same plane every other lane's pills use, labelled with the factor it applies. `Timeline.tsx` renders the lane only when `doc.cuts.length || doc.speed.length` - an empty Time lane on every untouched recording would be a row of nothing - and places it FIRST, nearest the filmstrip, because cuts and speed spans are the clip's own structure rather than a decoration laid over it.

Cuts share this lane's time but not its row: they are drawn across the whole track stack by `CutOverlay.tsx`, because a cut removes time from every lane at once.

## speedLabel

```ts
const speedLabel = (s: Speed) => `${+s.factor.toFixed(2)}x`;
```

`2x`, `0.5x`, `1.75x` - the factor with trailing zeros dropped. Module scope, not an inline arrow in the component body, so `RegionRows`' `React.memo` survives a playhead tick (the convention `Timeline.md`'s "Render hygiene" section records).

## TimeLane

```tsx
export const TimeLane: React.MemoExoticComponent<(props: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}) => JSX.Element>
```

A single `<RegionRows rows={1} ... rowClass="e-timerow" blkClass="e-spdblk">`, driven by its own `useRegionDrag` instance.

### Props

- `doc: EditDoc` - only `doc.speed` is read; the cuts in the same doc are `CutOverlay`'s.
- `dur: number` - the clip's duration, the basis for the pills' percentage geometry (the same one every other lane uses).
- `sel` / `onSel` - the editor's one selection; selecting a span opens `SpeedInspector` and arms Delete.
- `onApply` - the op sink; a drag commits exactly one `update_speed`.
- `track` - `Timeline`'s `.e-tlbody` ref, which `useRegionDrag` divides pointer deltas by.

### Behavior

**One row, always.** The ops keep speed spans disjoint (`normalize_speed` clamps each span to start at its predecessor's end), so they never need the layer stacking the zoom/FX/layout lanes use. `layer` is a constant `0`, and the row-stacking layer `useRegionDrag` reports on commit is dropped - which is why `update_speed` carries no `layer` field at all.

**Drag.** The shared pill mechanics: the body moves the span, the two `.e-zh` edge handles retime it, a local draft follows the pointer and exactly one `update_speed` is applied on release (and none at all for a bare click under the 3px threshold, which selects only). `useRegionDrag` clamps a drag against same-row neighbours, which here means the neighbouring speed spans - so a drag can never produce the overlap `normalize_speed` would then have to resolve behind the user's back.

**Look.** `.e-spdblk` sets `--pill-accent: var(--e-speed)`, the one lane accent the palette was missing; everything else about the pill (the raised plane, the 3px accent bar, the hairline, the hover/selected treatment, the `@container` label hiding on a narrow pill) comes from the shared rules in `timeline.css`, so the lane reads as the same instrument as the others.

### Notes

- `spans` is `useMemo`'d on `doc.speed` so `beginDrag` and `RegionRows`' memo stay stable across a playhead tick; `onCommit` is `useCallback`'d on `onApply` for the same reason (`useRegionDrag.md`).
- The lane's gutter label is "Time", and it brightens with `isSel(doc.speed)` like every other lane's.
