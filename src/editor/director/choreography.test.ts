import { describe, expect, it } from "vitest";
import { planStep, pace, dwellSettle } from "./choreography";

describe("planStep", () => {
  it("aims the zoom lane and presses for add_zoom_full", () => {
    expect(planStep({ op: "add_zoom_full", at_ms: 1200, dur_ms: 2000, scale: 2 })).toEqual({
      kind: "aim-timeline",
      lane: "zoom",
      ms: 1200,
      pressAfterMove: true,
    });
  });

  it("aims the effects lane and presses for add_effect", () => {
    expect(planStep({ op: "add_effect", kind: "spotlight", start_ms: 400, end_ms: 1400 })).toEqual({
      kind: "aim-timeline",
      lane: "fx",
      ms: 400,
      pressAfterMove: true,
    });
  });

  it("drags the trim-in handle to in_ms for set_trim (the 'out' leg is decided by the orchestration, not here)", () => {
    expect(planStep({ op: "set_trim", in_ms: 500, out_ms: 9000 })).toEqual({
      kind: "drag-trim",
      lane: "trim-in",
      ms: 500,
      pressAfterMove: true,
    });
  });

  it("holds position (kind 'none') for an op with no pill to land on", () => {
    expect(planStep({ op: "clear_zooms" })).toEqual({ kind: "none", lane: "zoom", pressAfterMove: false });
    expect(planStep({ op: "add_layout_seg", at_ms: 0, dur_ms: 1000, layout: "presenter" }).kind).toBe("none");
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
    expect(dwellSettle(16)).toEqual({ dwellMs: 80, settleMs: 120 });
  });

  it("floors the scaled values at 60ms even for a very long plan", () => {
    expect(dwellSettle(100)).toEqual({ dwellMs: 60, settleMs: 60 });
  });
});
