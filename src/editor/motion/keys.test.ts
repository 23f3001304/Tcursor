import { describe, expect, it } from "vitest";
import { evalKeys, isKeys } from "./keys";
import { ease } from "../timeline/model/layoutTrack";
import { CURVES, key, P, parsed } from "./keysFixture";

describe("evalKeys", () => {
  it("matches the Rust parity table to 1e-4", () => {
    for (const c of CURVES) {
      const k = parsed(c.wire);
      c.want.forEach((w, i) => expect(evalKeys(k, P[i])).toBeCloseTo(w, 4));
    }
  });

  it("draws today's smoothstep for the Soft curve, to 1e-3", () => {
    const k = parsed(CURVES[4].wire);
    for (let i = 0; i <= 100; i++) {
      const p = i / 100;
      expect(evalKeys(k, p)).toBeCloseTo(p * p * (3 - 2 * p), 3);
    }
  });

  it("clamps p but not the value - anticipation and overshoot survive", () => {
    const k = parsed(CURVES[5].wire);
    expect(evalKeys(k, -1)).toBe(evalKeys(k, 0));
    expect(evalKeys(k, 2)).toBe(evalKeys(k, 1));
    expect(evalKeys(k, 0.9)).toBeGreaterThan(1);
    expect(evalKeys({ keys: [key(0, 0, [0.2, -0.4]), key(1, 1, [0, 0], [-0.3, 0])] }, 0.2)).toBeLessThan(0);
  });

  it("holds through a hold segment and lerps through a linear one", () => {
    const k = parsed(CURVES[3].wire);
    expect(evalKeys(k, 0.425)).toBeCloseTo(0.5, 6);
    expect(evalKeys(k, 0.86)).toBe(1);
    expect(evalKeys(k, 0.99)).toBe(1);
  });
});

describe("isKeys", () => {
  it("is true only for a curve that really parses", () => {
    expect(isKeys(CURVES[0].wire)).toBe(true);
    expect(isKeys("keys(garbage)")).toBe(false);
    expect(isKeys("smooth")).toBe(false);
    expect(isKeys(null)).toBe(false);
    expect(isKeys(undefined)).toBe(false);
  });
});

describe("ease's keys arm", () => {
  it("evaluates a keys curve through evalKeys", () => {
    for (const c of CURVES) {
      const k = parsed(c.wire);
      for (const p of P) expect(ease(c.wire, p)).toBe(evalKeys(k, p));
      expect(ease(c.canon, 0.33)).toBe(evalKeys(k, 0.33));
    }
  });

  it("falls back to smooth on a malformed curve, as valid_easing coerces it", () => {
    for (const p of P) {
      expect(ease("keys(garbage)", p)).toBe(ease("smooth", p));
      expect(ease("keys(0 0 0 0 0 0 b)", p)).toBe(ease("smooth", p));
    }
  });
});
