import { describe, expect, it } from "vitest";
import { estimateEtaMs } from "./exportEta";

describe("estimateEtaMs", () => {
  it("returns null before there is any progress signal", () => {
    expect(estimateEtaMs(5_000, 0)).toBeNull();
  });

  it("projects the remaining time from a constant encode rate", () => {
    // 10s elapsed at 50% -> 10s total -> 10s remaining.
    expect(estimateEtaMs(10_000, 50)).toBe(10_000);
  });

  it("shrinks toward zero as pct approaches 100", () => {
    expect(estimateEtaMs(9_900, 99)).toBeCloseTo(100, 0);
  });

  it("never returns a negative remainder at/over 100%", () => {
    expect(estimateEtaMs(10_000, 100)).toBe(0);
  });
});
