import type { EditOp } from "../../shared/edit";
import type { Lane } from "./targets";

export type StepKind = "aim-timeline" | "drag-trim" | "none";

export interface StepPlan {
  kind: StepKind;
  lane: Lane;
  ms?: number;
  pressAfterMove: boolean;
}

export function planStep(op: EditOp): StepPlan {
  switch (op.op) {
    case "add_zoom_full":
      return { kind: "aim-timeline", lane: "zoom", ms: op.at_ms, pressAfterMove: true };
    case "add_effect":
      return { kind: "aim-timeline", lane: "fx", ms: op.start_ms, pressAfterMove: true };
    case "set_trim":
      return { kind: "drag-trim", lane: "trim-in", ms: op.in_ms, pressAfterMove: true };
    default:
      return { kind: "none", lane: "zoom", pressAfterMove: false };
  }
}

const MIN_TRAVEL_MS = 240,
  MS_PER_PX = 0.6,
  MAX_TRAVEL_MS = 700;
const BASE_DWELL_MS = 160,
  BASE_SETTLE_MS = 240,
  LONG_PLAN_STEPS = 8,
  MIN_SCALED_MS = 60;

export function pace(distancePx: number): { travelCapMs: number } {
  return { travelCapMs: Math.min(MAX_TRAVEL_MS, MIN_TRAVEL_MS + distancePx * MS_PER_PX) };
}

export function dwellSettle(stepCount: number): { dwellMs: number; settleMs: number } {
  const scale = stepCount > LONG_PLAN_STEPS ? LONG_PLAN_STEPS / stepCount : 1;
  return {
    dwellMs: Math.max(MIN_SCALED_MS, Math.round(BASE_DWELL_MS * scale)),
    settleMs: Math.max(MIN_SCALED_MS, Math.round(BASE_SETTLE_MS * scale)),
  };
}
