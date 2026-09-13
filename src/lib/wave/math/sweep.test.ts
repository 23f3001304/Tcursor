import { describe, it, expect } from "vitest";
import { SWEEP_PERIOD_S, TRAIL, headFor, progressAmp, sweepEnvelope, sweepHead } from "./sweep";

describe("sweepHead", () => {
  it("runs 0 -> 1 across one period and restarts at the left edge", () => {
    expect(sweepHead(0)).toBe(0);
    expect(sweepHead(SWEEP_PERIOD_S / 2)).toBeCloseTo(0.5, 10);
    expect(sweepHead(SWEEP_PERIOD_S - 1e-9)).toBeGreaterThan(0.99);
    expect(sweepHead(SWEEP_PERIOD_S)).toBeCloseTo(0, 10); // sawtooth, never a bounce back
    expect(sweepHead(SWEEP_PERIOD_S * 3.25)).toBeCloseTo(0.25, 10);
  });

  it("stays in 0..1 for a negative or degenerate clock", () => {
    expect(sweepHead(-0.3)).toBeGreaterThanOrEqual(0);
    expect(sweepHead(-0.3)).toBeLessThan(1);
    expect(sweepHead(1, 0)).toBe(0);
  });
});

describe("sweepEnvelope", () => {
  it("is flat ahead of the head - the wave only exists where work has passed", () => {
    expect(sweepEnvelope(0.7, 0.5)).toBe(0);
    expect(sweepEnvelope(1, 0.5)).toBe(0);
  });

  it("is full height at the head and dies out one trail behind it", () => {
    expect(sweepEnvelope(0.5, 0.5)).toBeCloseTo(1, 10);
    expect(sweepEnvelope(0.5 - TRAIL, 0.5)).toBe(0);
    expect(sweepEnvelope(0.5 - TRAIL / 2, 0.5)).toBeCloseTo(0.5, 10);
  });

  it("decays monotonically behind the head, with no corner at the trailing edge", () => {
    let prev = Infinity;
    for (let i = 0; i <= 20; i++) {
      const v = sweepEnvelope(0.9 - (i / 20) * TRAIL, 0.9);
      expect(v).toBeLessThanOrEqual(prev + 1e-12);
      prev = v;
    }
    expect(prev).toBe(0);
  });
});

describe("progressAmp", () => {
  it("decays the wave to dead flat exactly at 100%", () => {
    expect(progressAmp(0)).toBe(1);
    expect(progressAmp(50)).toBeCloseTo(0.5, 10);
    expect(progressAmp(100)).toBe(0);
  });

  it("clamps a stale or out-of-range percent instead of inverting", () => {
    expect(progressAmp(-40)).toBe(1);
    expect(progressAmp(180)).toBe(0);
  });
});

describe("headFor", () => {
  it("sweeps from the clock when the percent is unknown", () => {
    expect(headFor(SWEEP_PERIOD_S / 4, undefined)).toBeCloseTo(0.25, 10);
  });

  it("parks the head ON the percent when it is known, so the dot IS the marker", () => {
    expect(headFor(999, 37)).toBeCloseTo(0.37, 10);
    expect(headFor(0, 140)).toBe(1);
    expect(headFor(0, -5)).toBe(0);
  });
});
