import { describe, expect, it } from "vitest";
import { formatSpring, parseSpring, spring, SPRING_DEFAULT, springOf } from "./spring";

const PARITY: [number, number, number, number[]][] = [
  [170, 26, 1, [0.15422, 0.405341, 0.618152, 0.768274, 0.865271, 0.924838, 0.960285, 0.980989, 0.992996]],
  [300, 10, 1, [1.2169, 1.107563, 0.874016, 1.055725, 0.994093, 0.989035, 1.007699, 0.996468, 0.999043]],
  [170, 60, 1, [0.471265, 0.735163, 0.867478, 0.933845, 0.96716, 0.98391, 0.992357, 0.996643, 0.998844]],
  [100, 10, 1, [0.547465, 1.085405, 1.1451, 1.031899, 0.975458, 0.983249, 1.000326, 1.004673, 1.00202]],
];

const sweep = (k: number, c: number, m: number) =>
  Array.from({ length: 2001 }, (_, i) => spring(k, c, m, i / 2000));

describe("spring", () => {
  it("matches the Rust parity table to 1e-4", () => {
    for (const [k, c, m, want] of PARITY) {
      want.forEach((w, i) => {
        expect(spring(k, c, m, (i + 1) / 10)).toBeCloseTo(w, 4);
      });
    }
  });

  it("has exact endpoints on every branch", () => {
    for (const [k, c, m] of [
      [170, 26, 1],
      [300, 10, 1],
      [170, 60, 1],
      [2000, 0, 1],
      [1, 200, 10],
    ]) {
      expect(spring(k, c, m, 0)).toBe(0);
      expect(spring(k, c, m, 1)).toBe(1);
      expect(spring(k, c, m, 0.999)).toBeCloseTo(1, 3);
      expect(spring(k, c, m, -1)).toBe(0);
      expect(spring(k, c, m, 2)).toBe(1);
      expect(spring(k, c, m, NaN)).toBe(0);
    }
  });

  it("overshoots more the lower the damping ratio, and not at all past critical", () => {
    const peak = (k: number, c: number) => Math.max(...sweep(k, c, 1));
    expect(peak(300, 10)).toBeGreaterThan(peak(100, 10));
    expect(peak(100, 10)).toBeGreaterThan(peak(170, 26));
    expect(peak(170, 60)).toBeCloseTo(1, 4);
  });

  it("is monotone when critically damped or overdamped", () => {
    for (const [k, c] of [
      [170, 26.077],
      [170, 60],
      [1, 200],
    ]) {
      const v = sweep(k, c, 1);
      for (let i = 1; i < v.length; i++) expect(v[i]).toBeGreaterThanOrEqual(v[i - 1] - 1e-9);
    }
  });

  it("depends on the damping ratio alone, so the same zeta is the same curve", () => {
    for (let i = 1; i < 10; i++) {
      expect(spring(170, 26, 1, i / 10)).toBeCloseTo(spring(680, 52, 1, i / 10), 4);
    }
  });
});

describe("parseSpring / formatSpring", () => {
  it("round-trips and defaults mass", () => {
    expect(parseSpring("spring(170,26)")).toEqual([170, 26, 1]);
    expect(parseSpring("spring( 300 , 10 , 2 )")).toEqual([300, 10, 2]);
    expect(formatSpring(170, 26, 1)).toBe("spring(170.000,26.000,1.000)");
    expect(parseSpring(formatSpring(300, 10, 2))).toEqual([300, 10, 2]);
  });

  it("clamps out-of-range values and rejects malformed input", () => {
    expect(parseSpring("spring(99999,-3,99)")).toEqual([2000, 0, 10]);
    for (const bad of [
      "spring(170)",
      "spring(1,2,3,4)",
      "spring()",
      "spring(a,b)",
      "spring(1,2",
      "cubic(0,0,1,1)",
      "spring",
      "",
      null,
      undefined,
    ]) {
      expect(parseSpring(bad)).toBeNull();
    }
  });

  it("resolves the bare word to SPRING_DEFAULT and leaves other curves alone", () => {
    expect(springOf("spring")).toEqual(SPRING_DEFAULT);
    expect(springOf("spring(300,10)")).toEqual([300, 10, 1]);
    expect(springOf("smooth")).toBeNull();
    expect(springOf("cubic(0.25,0.1,0.25,1)")).toBeNull();
  });
});
