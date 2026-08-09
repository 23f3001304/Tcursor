import type { EditOp } from "../../lib/edit";
import type { Lane } from "./targets";

export type StepKind = "aim-timeline" | "sweep-lane" | "drag-trim" | "none";

/** Pure choreography plan for one AI-director edit op - what the fake pointer should do (see
 *  `DirectorPointer.tsx`) before/around the real `applyEditOp` call that actually performs it.
 *  `ms` is the PRIMARY aim target (absent for `sweep-lane`, which has no single point). `set_trim`
 *  can move both trim handles, but `planStep` only sees the op, not the doc's PRIOR trim - it
 *  plans the "in" leg here, and `useDirector.ts`'s orchestration (which HAS the current doc)
 *  decides the "out" leg itself, using the same `timelinePointForMs`, only when `out_ms` actually
 *  changed. */
export interface StepPlan { kind: StepKind; lane: Lane; ms?: number; pressAfterMove: boolean }

/** Maps one AI-director op to its `StepPlan` - the only op kinds the planner ever actually emits
 *  are `add_zoom_full`, `clear_zooms`, and `set_trim` (see `ai_plan`/`ops_from_json` on the Rust
 *  side); anything else (defensive - not currently emitted) holds the pointer's position and
 *  applies normally. */
export function planStep(op: EditOp): StepPlan {
  switch (op.op) {
    case "add_zoom_full": return { kind: "aim-timeline", lane: "zoom", ms: op.at_ms, pressAfterMove: true };
    case "clear_zooms": return { kind: "sweep-lane", lane: "zoom", pressAfterMove: false };
    case "set_trim": return { kind: "drag-trim", lane: "trim-in", ms: op.in_ms, pressAfterMove: true };
    default: return { kind: "none", lane: "zoom", pressAfterMove: false };
  }
}

const MIN_TRAVEL_MS = 240, MS_PER_PX = 0.6, MAX_TRAVEL_MS = 700;
const BASE_DWELL_MS = 160, BASE_SETTLE_MS = 240, LONG_PLAN_STEPS = 8, MIN_SCALED_MS = 60;

/** Travel-time cap (ms) for a `distancePx` pointer move. The spring (`DirectorPointer`'s
 *  `useSpring`) drives the actual motion; this only bounds how long `moveTo` waits for the
 *  "settled within 2px" check before resolving anyway, so a long-distance move can never stall
 *  the reveal. Min 240ms, +0.6ms/px, capped at 700ms. */
export function pace(distancePx: number): { travelCapMs: number } {
  return { travelCapMs: Math.min(MAX_TRAVEL_MS, MIN_TRAVEL_MS + distancePx * MS_PER_PX) };
}

/** Dwell-before-press and settle-after-apply durations for one step, scaled down once a plan runs
 *  longer than 8 steps (`8 / stepCount`) so a big reveal still finishes in roughly ~10s instead of
 *  stacking up linearly - each floors at 60ms so even a very long plan still visibly pauses
 *  between steps rather than flickering. */
export function dwellSettle(stepCount: number): { dwellMs: number; settleMs: number } {
  const scale = stepCount > LONG_PLAN_STEPS ? LONG_PLAN_STEPS / stepCount : 1;
  return {
    dwellMs: Math.max(MIN_SCALED_MS, Math.round(BASE_DWELL_MS * scale)),
    settleMs: Math.max(MIN_SCALED_MS, Math.round(BASE_SETTLE_MS * scale)),
  };
}
