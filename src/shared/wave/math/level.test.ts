// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import {
  AMP_MAX,
  AMP_MIN,
  CEIL_DB,
  FLOOR_DB,
  IDLE_DB,
  damp,
  dbFromRms,
  heightFromRms,
  levelFromRms,
} from "./level";

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
  it("is flat at silence and full height from the ceiling up, a taller slot's max included", () => {
    expect(heightFromRms(0)).toBe(AMP_MIN);
    expect(heightFromRms(Math.pow(10, CEIL_DB / 20))).toBeCloseTo(AMP_MAX, 10);
    expect(heightFromRms(1)).toBeCloseTo(AMP_MAX, 10);
    expect(heightFromRms(1, 34)).toBeCloseTo(34, 10);
  });

  it("is LOG mapped over a speech window: the halfway dB lands halfway up, not the halfway RMS", () => {
    const half = Math.pow(10, (FLOOR_DB + CEIL_DB) / 2 / 20);
    expect(heightFromRms(half)).toBeCloseTo((AMP_MIN + AMP_MAX) / 2, 6);
    expect(levelFromRms(half)).toBeCloseTo(0.5, 6);
    expect(levelFromRms(0.03)).toBeGreaterThan(0.45);
    expect(FLOOR_DB).toBeGreaterThanOrEqual(IDLE_DB);
  });

  it("clamps anything below the floor to flat", () => {
    expect(heightFromRms(Math.pow(10, (FLOOR_DB - 5) / 20))).toBe(AMP_MIN);
    expect(levelFromRms(0)).toBe(0);
  });
});

describe("damp", () => {
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
