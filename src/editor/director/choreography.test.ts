import { describe, expect, it } from "vitest";
import { planStep, pace, dwellSettle } from "./choreography";

describe("planStep", () => {
  it("aims the zoom lane and presses for add_zoom_full", () => {
    expect(planStep({ op: "add_zoom_full", at_ms: 1200, dur_ms: 2000, scale: 2 }))
      .toEqual({ kind: "aim-timeline", lane: "zoom", ms: 1200, pressAfterMove: true });
  });

  it("sweeps the zoom lane without a press for clear_zooms", () => {
    expect(planStep({ op: "clear_zooms" })).toEqual({ kind: "sweep-lane", lane: "zoom", pressAfterMove: false });
  });

  it("drags the trim-in handle to in_ms for set_trim (the 'out' leg is decided by the orchestration, not here)", () => {
    expect(planStep({ op: "set_trim", in_ms: 500, out_ms: 9000 }))
      .toEqual({ kind: "drag-trim", lane: "trim-in", ms: 500, pressAfterMove: true });
  });

  it("holds position (kind 'none') for any other op", () => {
    expect(planStep({ op: "remove_zoom", id: "z1" })).toEqual({ kind: "none", lane: "zoom", pressAfterMove: false });
    expect(planStep({ op: "add_effect", kind: "spotlight", start_ms: 0, end_ms: 1000 }).kind).toBe("none");
  });
});

describe("pace", () => {
  it("floors at the minimum travel time for a zero-distance move", () => {
    expect(pace(0).travelCapMs).toBe(240);
  });

  it("scales linearly with distance (0.6ms/px)", () => {
    expect(pace(100).travelCapMs).toBe(300);
  });

  it("caps at the maximum travel time for a long move", () => {
    expect(pace(10_000).travelCapMs).toBe(700);
    expect(pace(766.67).travelCapMs).toBeCloseTo(700, 0);
  });
});

describe("dwellSettle", () => {
  it("uses the base dwell/settle for a plan of 8 steps or fewer", () => {
    expect(dwellSettle(3)).toEqual({ dwellMs: 160, settleMs: 240 });
    expect(dwellSettle(8)).toEqual({ dwellMs: 160, settleMs: 240 });
  });

  it("scales dwell/settle down for a long plan (>8 steps)", () => {
    // 16 steps -> scale 8/16 = 0.5 -> 80ms dwell, 120ms settle
    expect(dwellSettle(16)).toEqual({ dwellMs: 80, settleMs: 120 });
  });

  it("floors the scaled values at 60ms even for a very long plan", () => {
    expect(dwellSettle(100)).toEqual({ dwellMs: 60, settleMs: 60 });
  });
});
