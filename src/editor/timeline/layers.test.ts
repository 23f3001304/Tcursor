import { describe, expect, it } from "vitest";
import { layoutRegions } from "./layers";

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
