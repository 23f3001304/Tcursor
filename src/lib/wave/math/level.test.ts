import { describe, it, expect } from "vitest";
import { AMP_MAX, AMP_MIN, IDLE_DB, damp, dbFromRms, heightFromRms } from "./level";

describe("dbFromRms", () => {
  it("floors digital silence well under the idle threshold instead of returning -Infinity", () => {
    expect(Number.isFinite(dbFromRms(0))).toBe(true);
    expect(dbFromRms(0)).toBeLessThan(IDLE_DB);
    expect(dbFromRms(-1)).toBe(dbFromRms(0));
  });

  it("maps full scale to 0 dBFS and each halving to -6 dB", () => {
    expect(dbFromRms(1)).toBeCloseTo(0, 10);
    expect(dbFromRms(0.5)).toBeCloseTo(-6.0206, 3);
    expect(dbFromRms(0.25)).toBeCloseTo(-12.0412, 3);
  });
});

describe("heightFromRms", () => {
  it("is flat at silence and full height at 0 dBFS", () => {
    expect(heightFromRms(0)).toBe(AMP_MIN);
    expect(heightFromRms(1)).toBeCloseTo(AMP_MAX, 10);
  });

  it("is LOG mapped: the halfway dB lands halfway up, not the halfway RMS", () => {
    const half = Math.pow(10, IDLE_DB / 2 / 20); // -25 dBFS
    expect(heightFromRms(half)).toBeCloseTo((AMP_MIN + AMP_MAX) / 2, 6);
    expect(heightFromRms(0.5)).toBeGreaterThan((AMP_MIN + AMP_MAX) * 0.7); // -6 dB is still loud
  });

  it("clamps anything below the idle floor to flat", () => {
    expect(heightFromRms(Math.pow(10, (IDLE_DB - 20) / 20))).toBe(AMP_MIN);
  });
});

describe("damp", () => {
  // `n` steps of `total / n` seconds each, so every call covers EXACTLY `total` seconds and the
  // frame-rate comparison below is about the solver, not about loop rounding.
  const step = (tau: number, total: number, n: number) => {
    let s = { value: 0, vel: 0 };
    for (let i = 0; i < n; i++) s = damp(s.value, s.vel, 24, tau, total / n);
    return s.value;
  };

  it("never overshoots its target", () => {
    let s = { value: 0, vel: 0 };
    for (let i = 0; i < 200; i++) {
      s = damp(s.value, s.vel, 24, 0.08, 1 / 60);
      expect(s.value).toBeLessThanOrEqual(24 + 1e-9);
    }
  });

  it("lags by tau: ~26% of a step after one tau, ~95% after five", () => {
    expect(step(0.08, 0.08, 1) / 24).toBeCloseTo(1 - 2 * Math.exp(-1), 6);
    expect(step(0.08, 0.4, 96) / 24).toBeGreaterThan(0.95);
  });

  it("is frame-rate independent: many small steps land where one big step does", () => {
    expect(step(0.08, 0.08, 20)).toBeCloseTo(step(0.08, 0.08, 1), 9);
    expect(step(0.08, 0.16, 5)).toBeCloseTo(step(0.08, 0.16, 40), 9);
  });

  it("is a no-op for a zero or negative dt", () => {
    expect(damp(3, 1, 24, 0.08, 0)).toEqual({ value: 3, vel: 1 });
  });
});
