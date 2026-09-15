import { describe, expect, it } from "vitest";
import { MIN_PILL_PCT, PILL_EDGE_INSET_PCT, PILL_GAP_PCT, pillLeftPct, pillWidthPct } from "./pillGeometry";

describe("pillLeftPct", () => {
  it("insets a region starting at the clip's own beginning instead of sitting on the true 0% edge (gate-feedback item 1)", () => {
    expect(pillLeftPct(0, 10_000)).toBe(PILL_EDGE_INSET_PCT);
  });

  it("leaves a mid-track region's left offset unaffected - the inset only applies at the true edge", () => {
    expect(pillLeftPct(6_000, 10_000)).toBeCloseTo(60, 5);
  });

  it("returns 0 for a zero/negative duration rather than dividing by zero", () => {
    expect(pillLeftPct(0, 0)).toBe(0);
  });
});

describe("pillWidthPct", () => {
  it("insets a full-clip region's BOTH edges instead of reaching the true 0%/100% (extends T11 to the left, ux audit #26 + gate-feedback item 1)", () => {
    const left = pillLeftPct(0, 10_000);
    const w = pillWidthPct(0, 10_000, 10_000);
    expect(left).toBeCloseTo(PILL_EDGE_INSET_PCT, 5);
    expect(w).toBeCloseTo(100 - PILL_EDGE_INSET_PCT * 2, 5);
    expect(left + w).toBeCloseTo(100 - PILL_EDGE_INSET_PCT, 5);
    expect(left).toBeGreaterThan(0);
    expect(left + w).toBeLessThan(100);
  });

  it("insets any region whose end lands on the clip's own duration, not just start=0 ones", () => {
    const start = 6_000,
      end = 10_000,
      dur = 10_000;
    const leftPct = pillLeftPct(start, dur);
    const w = pillWidthPct(start, end, dur);
    expect(leftPct + w).toBeCloseTo(100 - PILL_EDGE_INSET_PCT, 5);
  });

  it("leaves a mid-track region's width unaffected - the cap only bites at an edge", () => {
    expect(pillWidthPct(1_000, 5_000, 10_000)).toBeCloseTo(40, 5);
  });

  it("still floors tiny regions at MIN_PILL_PCT so they stay draggable, even when left-anchored", () => {
    expect(pillWidthPct(5_000, 5_010, 10_000)).toBe(MIN_PILL_PCT);
    expect(pillWidthPct(0, 10, 10_000)).toBe(MIN_PILL_PCT);
  });

  it("returns 0 for a zero/negative duration rather than dividing by zero", () => {
    expect(pillWidthPct(0, 100, 0)).toBe(0);
  });

  it("accepts an explicit leftPct so a caller stays consistent with a separately-computed pillLeftPct call", () => {
    const leftPct = pillLeftPct(0, 10_000);
    expect(pillWidthPct(0, 10_000, 10_000, leftPct)).toBe(pillWidthPct(0, 10_000, 10_000));
  });
});

describe("pillWidthPct with a neighbour", () => {
  it("floors a tiny pill only as far as the next pill in its row, so dense captions never overlap", () => {
    // 100 s track: a 500 ms caption is 0.5%, the floor would make it 2.5%, the next one starts 1.1% later.
    const w = pillWidthPct(10_000, 10_500, 100_000, undefined, 11_100);
    expect(w).toBeCloseTo(1.1 - PILL_GAP_PCT, 5);
    expect(w).toBeGreaterThan(0.5);
  });

  it("keeps the full floor when the next pill is far enough away, and never shrinks a pill below its real width", () => {
    expect(pillWidthPct(10_000, 10_500, 100_000, undefined, 20_000)).toBe(MIN_PILL_PCT);
    expect(pillWidthPct(10_000, 14_000, 100_000, undefined, 14_050)).toBeCloseTo(4, 5);
  });
});
