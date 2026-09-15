# src/editor/timeline/model/layers.ts

## layoutRegions

```ts
export function layoutRegions<T extends { start_ms: number; end_ms: number }>(regions: T[]): (T & { layer: number })[]
```

Greedy interval-partitioning: sort the regions by start, then place each on the lowest layer whose previous region ends at or before this one's start. Overlapping regions land on new layers, so the timeline can stack overlapping effects on separate rows. Two regions that merely touch at a boundary (`prev.end_ms <= start_ms`) share a layer. Returns each region with its assigned `layer` (0-based), leaving the input order to the caller.

## MAX_RAMP_PCT

```ts
export const MAX_RAMP_PCT = 40;
```

The cap on a layout pill's fade-ramp width, as a percentage of the pill. `40` is chosen so the two ramps can never meet: even a segment whose transitions are longer than its own span keeps a solid middle and still reads as a pill with two ends rather than one continuous gradient.

## transitionRampPct

```ts
export function transitionRampPct(transitionMs: number, spanMs: number): number
```

How wide a layout pill's in- or out-fade ramp should be: the transition's share of the segment's own span, capped at `MAX_RAMP_PCT`.

Returns `0` for a hard cut (`transitionMs === 0`) and for a degenerate span (`0` or negative), so the gradient is simply invisible and a pill with no transitions renders exactly as it always has.

### Used by

- `src/editor/timeline/Timeline.tsx` - written inline onto each `.e-layblk` as the `--fin` / `--fout` custom properties, which `timeline.css` turns into the two end gradients.
