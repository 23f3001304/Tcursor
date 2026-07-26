# src/editor/shell/exportEta.ts

Pure ETA math for `ExportProgress`, kept in its own file so the estimate can be unit-tested (`exportEta.test.ts`) without rendering a component.

## estimateEtaMs

```ts
export function estimateEtaMs(elapsedMs: number, pct: number): number | null
```

Rough ETA (ms remaining) from elapsed time and percent complete, assuming a constant encode rate.

### Inputs

- `elapsedMs` (`number`) - milliseconds since the export started (caller-tracked wall clock).
- `pct` (`number`) - progress percentage, 0-100, from the `export-progress` event.

### Returns

`number | null` - the estimated remaining milliseconds (`elapsed / pct * (100 - pct)`, never negative), or `null` when `pct <= 0` - before the first progress tick there is no rate to project from, and returning `null` (rather than `Infinity` or `0`) lets the caller show "Estimating..." instead of a wild or misleading early number.

### Implementation

`totalMs = elapsedMs / pct * 100`; return `max(0, totalMs - elapsedMs)`. Pure linear projection - no smoothing/averaging across ticks, so the estimate can jump around early in an export before settling as `pct` climbs.

### Behaviors

- Returns `null` at `pct <= 0`.
- `estimateEtaMs(10_000, 50) === 10_000` - 10s elapsed at 50% projects to 10s more.
- Approaches `0` as `pct` approaches 100; exactly `0` at `pct === 100`.

### Used by

- `ExportProgress` (`src/editor/shell/ExportProgress.tsx`) - recomputed on every `pct` change while exporting, from an internally-tracked start timestamp.
