# src/editor/timeline/layers.ts

## layoutRegions

```ts
export function layoutRegions<T extends { start_ms: number; end_ms: number }>(regions: T[]): (T & { layer: number })[]
```

Greedy interval-partitioning: sort the regions by start, then place each on the lowest layer whose previous region ends at or before this one's start. Overlapping regions land on new layers, so the timeline can stack overlapping effects on separate rows. Two regions that merely touch at a boundary (`prev.end_ms <= start_ms`) share a layer. Returns each region with its assigned `layer` (0-based), leaving the input order to the caller.
