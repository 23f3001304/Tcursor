# src/shared/math/remapPlan.ts

The tail of the TS mirror of `export::remap` (`docs/api/src-tauri/src/export/remap.md`), split out of `remap.ts` because that file is at its 200-line budget. One function: the frame-plan reader, the only one that answers about the PLAN rather than about the clip and output clocks. Same f64 expression order as the Rust side, so both agree to the bit; the parity table lives with the other remap tables in `remapClips.test.ts`.

## planBoundaries

```ts
export function planBoundaries(m: TimeMap, fps: number): number[]
```

Mirrors `TimeMap::plan_boundaries`: the ascending `framePlan` indices at which a segment opens that is NOT contiguous in source time with the one before it, walked with the same `frameBounds` arithmetic and the same per-segment entry count (`floor((kEnd - kStart) / factor) + 1`), so the running total is exactly each segment's first plan index. `lastEnd` is the `clipEnd` of the last segment that PRODUCED entries, so a segment with no frames at this rate is skipped without breaking the join; index 0 is never pushed.

The export snaps the cursor on exactly these entries (`FrameRenderer::walk_plan`), which is why keying on the plan and not on the output clock matters: a segment whose output length is fractional (0..1533 at 2x ends at 766.5 ms, while `ms(23)` is 766) puts the join between two whole output milliseconds, and `crossesBoundary(ms(j - 1), ms(j))` then answers one entry late. On this side it exists so the parity table pins the same list on both, and for the plan walk Batch 4 needs; the preview's own transport still asks `crossesBoundary`, which takes two output times and no plan.
