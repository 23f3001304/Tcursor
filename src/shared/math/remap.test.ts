import { describe, expect, it } from "vitest";
import {
  buildTimeMap,
  clipOf,
  crossesBoundary,
  factorAt,
  framePlan,
  outDurMs,
  outOf,
  resolveTrim,
} from "./remap";
import { fixtureMap } from "./remap.fixture";

const trimFrameBounds = (inMs: number, outMs: number, fps: number): [number, number] => {
  const kIn = Math.floor((inMs * fps) / 1000);
  return [kIn, Math.max(Math.floor((outMs * fps) / 1000), kIn)];
};
const range = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i);

describe("TimeMap (parity with export::remap)", () => {
  it("segments are the kept pieces with their factors and output starts", () => {
    const m = fixtureMap();
    expect(m.segments.map((s) => [s.clipStart, s.clipEnd, s.factor, s.outStart])).toEqual([
      [500, 1000, 1, 0],
      [2000, 2500, 1, 500],
      [2500, 3500, 2, 1000],
      [3500, 4000, 1, 1500],
      [4500, 6000, 1, 2000],
      [6000, 8000, 0.5, 3500],
      [8000, 9000, 1, 7500],
    ]);
    expect(outDurMs(m)).toBe(8500);
    expect(m.plain).toBe(false);
  });

  it("outOf is the parity table", () => {
    const m = fixtureMap();
    for (const [clip, out] of [
      [0, 0],
      [500, 0],
      [1200, 500],
      [2000, 500],
      [3000, 1250],
      [3500, 1500],
      [4250, 2000],
      [5000, 2500],
      [7000, 5500],
      [9000, 8500],
      [9999, 8500],
    ])
      expect(outOf(m, clip), `outOf(${clip})`).toBe(out);
  });

  it("clipOf is the parity table and inverts outOf on kept ranges", () => {
    const m = fixtureMap();
    for (const [out, clip] of [
      [0, 500],
      [250, 750],
      [500, 2000],
      [1250, 3000],
      [1500, 3500],
      [2000, 4500],
      [5500, 7000],
      [8500, 9000],
      [9000, 9000],
    ])
      expect(clipOf(m, out), `clipOf(${out})`).toBe(clip);
    for (const t of [500, 900, 2200, 3000, 3900, 5000, 7500, 8999])
      expect(clipOf(m, outOf(m, t)), `round trip ${t}`).toBe(t);
  });

  it("the 10fps frame plan keeps the trim edges and removes exactly the cut frames", () => {
    const plan = framePlan(fixtureMap(), 10);
    const expected = [
      ...range(5, 9),
      ...range(20, 24),
      25,
      27,
      29,
      31,
      33,
      ...range(35, 39),
      ...range(45, 59),
      ...range(60, 79)
        .flatMap((k) => [k, k])
        .slice(0, 39),
      ...range(80, 90),
    ];
    expect(plan).toEqual(expected);
    expect(plan.length).toBe(85);
    for (let i = 1; i < plan.length; i++) expect(plan[i]).toBeGreaterThanOrEqual(plan[i - 1]);
  });

  it("a trim-only map reproduces trim_frame_bounds exactly", () => {
    const full = 12_345;
    for (const [i, o, fps] of [
      [0, 0, 60],
      [1, 5000, 60],
      [333, 9999, 30],
      [0, 12_345, 24],
    ]) {
      const m = buildTimeMap({ in_ms: i, out_ms: o }, [], [], [], full);
      const [kIn, kLast] = trimFrameBounds(...resolveTrim({ in_ms: i, out_ms: o }, full), fps);
      expect(framePlan(m, fps), `trim ${i}..${o} @ ${fps}`).toEqual(range(kIn, kLast));
      expect(m.plain).toBe(true);
    }
    expect(framePlan(buildTimeMap({ in_ms: 700, out_ms: 700 }, [], [], [], full), 60)).toEqual([]);
  });

  it("overlapping and touching cuts merge and a cut inside a speed span wins", () => {
    const m = buildTimeMap(
      { in_ms: 0, out_ms: 0 },
      [
        { id: "a", start_ms: 100, end_ms: 300 },
        { id: "b", start_ms: 300, end_ms: 400 },
        { id: "c", start_ms: 250, end_ms: 350 },
      ],
      [{ id: "s", start_ms: 0, end_ms: 1000, factor: 2 }],
      [],
      1000,
    );
    expect(m.segments.map((s) => [s.clipStart, s.clipEnd, s.factor])).toEqual([
      [0, 100, 2],
      [400, 1000, 2],
    ]);
    expect(crossesBoundary(m, 49, 50)).toBe(true);
    expect(crossesBoundary(m, 10, 20)).toBe(false);
    expect(factorAt(m, 50)).toBe(2);
    expect(factorAt(m, 350)).toBe(1);
  });

  it("speed spans are clamped against each other and into range", () => {
    const m = buildTimeMap(
      { in_ms: 0, out_ms: 0 },
      [],
      [
        { id: "a", start_ms: 100, end_ms: 600, factor: 40 },
        { id: "b", start_ms: 400, end_ms: 800, factor: 0.01 },
      ],
      [],
      1000,
    );
    expect(m.segments.map((s) => [s.clipStart, s.clipEnd, s.factor])).toEqual([
      [0, 100, 1],
      [100, 600, 8],
      [600, 800, 0.25],
      [800, 1000, 1],
    ]);
  });

  it("a cut covering everything leaves no segments and no frames", () => {
    const m = buildTimeMap({ in_ms: 0, out_ms: 0 }, [{ id: "x", start_ms: 0, end_ms: 5000 }], [], [], 5000);
    expect(m.segments).toEqual([]);
    expect(outDurMs(m)).toBe(0);
    expect(framePlan(m, 60)).toEqual([]);
    expect(clipOf(m, 0)).toBe(0);
    expect(crossesBoundary(m, 0, 10)).toBe(false);
  });
});
