import { describe, expect, it } from "vitest";
import { layoutRegions, MAX_RAMP_PCT, transitionRampPct } from "./layers";

describe("layoutRegions", () => {
  it("preserves explicit layer assignments on regions", () => {
    const regions = [
      { id: "z1", start_ms: 0, end_ms: 1000, layer: 2 },
      { id: "z2", start_ms: 500, end_ms: 1500, layer: 1 },
    ];
    const laid = layoutRegions(regions);
    expect(laid.find((r) => r.id === "z1")?.layer).toBe(2);
    expect(laid.find((r) => r.id === "z2")?.layer).toBe(1);
  });

  it("assigns non-overlapping layers for unassigned regions", () => {
    const regions = [
      { id: "z1", start_ms: 0, end_ms: 1000 },
      { id: "z2", start_ms: 500, end_ms: 1500 },
      { id: "z3", start_ms: 2000, end_ms: 3000 },
    ];
    const laid = layoutRegions(regions);
    expect(laid.find((r) => r.id === "z1")?.layer).toBe(0);
    expect(laid.find((r) => r.id === "z2")?.layer).toBe(1);
    expect(laid.find((r) => r.id === "z3")?.layer).toBe(0);
  });
});

describe("transitionRampPct (T33 - layout pill fade ramps)", () => {
  it("is the transition's share of the pill", () => {
    expect(transitionRampPct(350, 2000)).toBeCloseTo(17.5, 6);
    expect(transitionRampPct(200, 1000)).toBeCloseTo(20, 6);
  });

  it("caps at 40% so the two ramps can never meet in the middle", () => {
    expect(transitionRampPct(900, 1000)).toBe(MAX_RAMP_PCT);
    expect(transitionRampPct(5000, 1000)).toBe(MAX_RAMP_PCT);
    expect(2 * MAX_RAMP_PCT).toBeLessThan(100);
  });

  it("is zero for a hard cut or a degenerate span", () => {
    expect(transitionRampPct(0, 2000)).toBe(0);
    expect(transitionRampPct(350, 0)).toBe(0);
    expect(transitionRampPct(350, -5)).toBe(0);
  });
});
