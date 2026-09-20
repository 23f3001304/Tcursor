# src/shared/math/remapPlan.ts

The tail of the TS mirror of `export::remap` (`docs/api/src-tauri/src/export/remap.md`), split out of `remap.ts` because that file is at its 200-line budget: `planBoundaries`, the frame-plan reader that answers about the PLAN rather than about the clip and output clocks, and the `ClipSpan`/`clipSpans` pair added beside it once Batch 4 needed to know where each clip's own run of frames begins. Same f64 expression order as the Rust side, so both agree to the bit; the parity table lives with the other remap tables in `remapClips.test.ts`.

## planBoundaries

```ts
export function planBoundaries(m: TimeMap, fps: number): number[]
```

Mirrors `TimeMap::plan_boundaries`: the ascending `framePlan` indices at which a segment opens that is NOT contiguous in source time with the one before it, walked with the same `frameBounds` arithmetic and the same per-segment entry count (`floor((kEnd - kStart) / factor) + 1`), so the running total is exactly each segment's first plan index. `lastEnd` is the `clipEnd` of the last segment that PRODUCED entries, so a segment with no frames at this rate is skipped without breaking the join; index 0 is never pushed.

The export snaps the cursor on exactly these entries (`FrameRenderer::walk_plan`), which is why keying on the plan and not on the output clock matters: a segment whose output length is fractional only puts the join between two whole output milliseconds when the NEXT segment is not contiguous with it. `remapClips.test.ts`'s own `fractional` fixture ends a 0..1533 speed span at 2x at output 766.5 ms and runs straight into the untouched rest of the clip, so the two segments still meet at clip time 1533 and `planBoundaries(30)` is rightly empty on it; give that same span a cut of 1533..2000 instead and the new segment's first entry lands at plan index 23, while `ms(23)` (`23 * 1000 / 30`) is 766, half a millisecond short of the true 766.5, so `crossesBoundary(ms(j - 1), ms(j))` answered false there and only fired at j = 24, one entry late. On this side `planBoundaries` exists so the parity table pins the same list on both, and for the plan walk Batch 4 needs; the preview's own transport never calls `crossesBoundary` at all, a multi-clip document stepping by segment in `clipTick` (`src/editor/stage/transport/outClock.ts`) and a single-clip one seeking through `playbackAction` (`src/editor/stage/transport/playback.ts`), neither asking a boundary predicate of two output times.

## ClipSpan

```ts
export interface ClipSpan {
  clip: number;
  planStart: number;
  planLen: number;
  firstK: number;
}
```

Mirrors Rust `ClipSpan` (`docs/api/src-tauri/src/export/remap_spans.md`).

## clipSpans

```ts
export function clipSpans(m: TimeMap, fps: number): ClipSpan[]
```

Mirrors `TimeMap::clip_spans`.
