// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { evalCubic, formatCubic, parseCubic } from "./cubicBezier";
import { ease } from "../../editor/timeline/model/layoutTrack";

describe("parseCubic", () => {
  it("parses well-formed and whitespaced forms", () => {
    expect(parseCubic("cubic(0.25,0.1,0.25,1)")).toEqual([0.25, 0.1, 0.25, 1]);
    expect(parseCubic("  cubic( 0.42 , 0 , 0.58 , 1 ) ")).toEqual([0.42, 0, 0.58, 1]);
  });

  it("rejects garbage, exactly like the Rust parser", () => {
    for (const s of [
      "smooth",
      "cubic(",
      "cubic()",
      "cubic(1,2,3)",
      "cubic(1,2,3,4,5)",
      "cubic(a,b,c,d)",
      "cubic(0.1,0.2,0.3,)",
      "bezier(0,0,1,1)",
      "",
      null,
      undefined,
    ]) {
      expect(parseCubic(s as string)).toBeNull();
    }
  });

  it("clamps x into 0..1 but leaves y free to overshoot", () => {
    expect(parseCubic("cubic(-3,-2,9,4)")).toEqual([0, -2, 1, 4]);
  });
});

describe("formatCubic", () => {
  it("emits the same canonical text Rust's format_cubic does, and re-parses", () => {
    expect(formatCubic(0.25, 0.1, 0.25, 1)).toBe("cubic(0.250,0.100,0.250,1.000)");
    expect(parseCubic(formatCubic(0.25, 0.1, 0.25, 1))).toEqual([0.25, 0.1, 0.25, 1]);
  });
});

describe("evalCubic (parity with Rust export::cubic::eval)", () => {
  it("has exact endpoints and clamps its input", () => {
    for (const c of [
      [0.25, 0.1, 0.25, 1],
      [0, 0, 1, 1],
      [0.34, 1.56, 0.64, 1],
    ] as const) {
      expect(evalCubic(c[0], c[1], c[2], c[3], 0)).toBe(0);
      expect(evalCubic(c[0], c[1], c[2], c[3], 1)).toBe(1);
      expect(evalCubic(c[0], c[1], c[2], c[3], -5)).toBe(0);
      expect(evalCubic(c[0], c[1], c[2], c[3], 5)).toBe(1);
    }
  });

  it("matches the CSS ease curve at the same probe points the Rust test pins", () => {
    expect(evalCubic(0.25, 0.1, 0.25, 1, 0.5)).toBeCloseTo(0.802403, 3);
    expect(evalCubic(0.25, 0.1, 0.25, 1, 0.25)).toBeCloseTo(0.408511, 3);
    expect(evalCubic(0.42, 0, 0.58, 1, 0.5)).toBeCloseTo(0.5, 4);
    for (const p of [0.1, 0.3, 0.5, 0.9]) expect(evalCubic(1 / 3, 1 / 3, 2 / 3, 2 / 3, p)).toBeCloseTo(p, 3);
  });

  it("is monotonic in x for in-range handles", () => {
    for (const [x1, x2] of [
      [0, 1],
      [1, 0],
      [0, 0],
      [1, 1],
      [0.25, 0.25],
    ]) {
      let prev = -1;
      for (let i = 0; i <= 100; i++) {
        const v = evalCubic(x1, 0, x2, 1, i / 100);
        expect(v).toBeGreaterThanOrEqual(prev - 1e-5);
        prev = v;
      }
    }
  });

  it("overshoots past 1 when a handle does, and still lands on 1", () => {
    expect(evalCubic(0.34, 1.56, 0.64, 1, 0.7)).toBeGreaterThan(1);
    expect(evalCubic(0.34, 1.56, 0.64, 1, 1)).toBe(1);
  });
});

describe("ease() dispatches custom cubics", () => {
  it("routes a cubic(...) wire name through evalCubic", () => {
    expect(ease("cubic(0.25,0.1,0.25,1)", 0.5)).toBeCloseTo(0.802403, 3);
  });

  it("leaves the six named curves untouched and still defaults garbage to smooth", () => {
    expect(ease("linear", 0.37)).toBeCloseTo(0.37, 6);
    expect(ease("ease_in", 0.5)).toBeCloseTo(0.25, 6);
    expect(ease("cubic(1,2)", 0.5)).toBeCloseTo(0.5, 6);
    expect(ease("wobble", 0.25)).toBeCloseTo(0.25 * 0.25 * (3 - 2 * 0.25), 6);
  });
});
