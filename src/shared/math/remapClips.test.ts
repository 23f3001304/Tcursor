import { describe, expect, it } from "vitest";
import { buildTimeMap, clipOf, clipOutMs, crossesBoundary, framePlan, outDurMs, outOf } from "./remap";
import { clipsFixtureMap, fixtureMap, fixtureParts } from "./remap.fixture";
import { clipSpans, planBoundaries } from "./remapPlan";

const range = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i);

describe("TimeMap with clips (parity with export::remap clips_fixture)", () => {
  it("clips in output order concatenate their kept pieces", () => {
    const m = clipsFixtureMap();
    expect(m.segments.map((s) => [s.clip, s.clipStart, s.clipEnd, s.factor, s.outStart])).toEqual([
      [0, 6000, 8000, 0.5, 0],
      [0, 8000, 9000, 1, 4000],
      [1, 500, 1000, 1, 5000],
      [1, 2000, 2500, 1, 5500],
      [1, 2500, 3500, 2, 6000],
      [1, 3500, 4000, 1, 6500],
    ]);
    expect(outDurMs(m)).toBe(7000);
    expect(m.plain).toBe(false);
    expect([clipOutMs(m, 0), clipOutMs(m, 1), clipOutMs(m, 2)]).toEqual([5000, 2000, 0]);
  });

  it("outOf and clipOf are the clips parity table", () => {
    const m = clipsFixtureMap();
    for (const [clip, out] of [
      [0, 5000],
      [500, 5000],
      [750, 5250],
      [1500, 5500],
      [2250, 5750],
      [3000, 6250],
      [3750, 6750],
      [4000, 0],
      [4250, 0],
      [6000, 0],
      [7000, 2000],
      [8000, 4000],
      [8500, 4500],
      [9000, 7000],
      [9999, 7000],
    ])
      expect(outOf(m, clip), `outOf(${clip})`).toBe(out);
    for (const [out, clip] of [
      [0, 6000],
      [1000, 6500],
      [4000, 8000],
      [4500, 8500],
      [5000, 500],
      [5250, 750],
      [5500, 2000],
      [6000, 2500],
      [6250, 3000],
      [6500, 3500],
      [6750, 3750],
      [7000, 4000],
    ])
      expect(clipOf(m, out), `clipOf(${out})`).toBe(clip);
  });

  it("a reordered frame plan is the concatenation and is not monotone", () => {
    const plan = framePlan(clipsFixtureMap(), 10);
    expect(plan).toEqual([
      ...range(60, 79)
        .flatMap((k) => [k, k])
        .slice(0, 39),
      ...range(80, 89),
      ...range(5, 9),
      ...range(20, 24),
      25,
      27,
      29,
      31,
      33,
      ...range(35, 40),
    ]);
    expect(plan.some((k, i) => i > 0 && plan[i - 1] > k)).toBe(true);
  });

  it("crossesBoundary is true only across a non-contiguous segment join", () => {
    const base: [number, number, boolean][] = [
      [499, 500, true],
      [999, 1000, false],
      [1499, 1500, false],
      [1999, 2000, true],
      [3499, 3500, false],
      [7499, 7500, false],
      [0, 0, false],
    ];
    const m = fixtureMap();
    for (const [a, b, want] of base) expect(crossesBoundary(m, a, b), `base (${a},${b})`).toBe(want);
    const clips: [number, number, boolean][] = [
      [3999, 4000, false],
      [4999, 5000, true],
      [5499, 5500, true],
      [5999, 6000, false],
      [6499, 6500, false],
      [100, 101, false],
    ];
    const c = clipsFixtureMap();
    for (const [a, b, want] of clips) expect(crossesBoundary(c, a, b), `clips (${a},${b})`).toBe(want);
  });

  it("planBoundaries are the plan indices that open a non-contiguous segment", () => {
    expect(planBoundaries(fixtureMap(), 10)).toEqual([5, 20]);
    expect(planBoundaries(clipsFixtureMap(), 10)).toEqual([49, 54]);
    const cut = (a: number, b: number) =>
      buildTimeMap({ in_ms: 0, out_ms: 0 }, [{ id: "x", start_ms: a, end_ms: b }], [], [], 10_000);
    const subFrame = cut(1003, 1015);
    expect(planBoundaries(subFrame, 30)).toEqual([31]);
    expect(framePlan(subFrame, 30)).toEqual(range(0, 300));
    expect(planBoundaries(cut(0, 10_000), 30)).toEqual([]);
    const fractional = buildTimeMap(
      { in_ms: 0, out_ms: 0 },
      [],
      [{ id: "s", start_ms: 0, end_ms: 1533, factor: 2 }],
      [],
      10_000,
    );
    expect(planBoundaries(fractional, 30)).toEqual([]);
  });

  it("a segment carries the index of its clip in the list as written", () => {
    const p = fixtureParts();
    const clip = (id: string, a: number, b: number) => ({
      id,
      src_in_ms: a,
      src_out_ms: b,
      transition_in_ms: 0,
    });
    const m = buildTimeMap(
      p.trim,
      [...p.cuts],
      [...p.speed],
      [clip("cl1", 6000, 9000), clip("bad", 6000, 2000), clip("cl0", 500, 4000)],
      10_000,
    );
    expect(m.segments.map((s) => [s.clip, s.clipStart, s.clipEnd, s.factor, s.outStart])).toEqual([
      [0, 6000, 8000, 0.5, 0],
      [0, 8000, 9000, 1, 4000],
      [2, 500, 1000, 1, 5000],
      [2, 2000, 2500, 1, 5500],
      [2, 2500, 3500, 2, 6000],
      [2, 3500, 4000, 1, 6500],
    ]);
    expect([clipOutMs(m, 0), clipOutMs(m, 1), clipOutMs(m, 2)]).toEqual([5000, 0, 2000]);
  });

  it("an empty clip list is the trim and one clip narrower than the trim is not plain", () => {
    const trim = { in_ms: 500, out_ms: 9000 };
    const one = (a: number, b: number) =>
      buildTimeMap(trim, [], [], [{ id: "cl0", src_in_ms: a, src_out_ms: b, transition_in_ms: 0 }], 10_000);
    expect(buildTimeMap(trim, [], [], [], 10_000).segments).toEqual(one(500, 9000).segments);
    expect(one(500, 9000).plain).toBe(true);
    expect(one(500, 8000).plain).toBe(false);
    expect(one(6000, 2000).segments).toEqual([]);
  });
});

describe("clipSpans (parity with export::remap_spans)", () => {
  const trim = { in_ms: 500, out_ms: 9000 };
  const clip = (id: string, a: number, b: number) => ({
    id,
    src_in_ms: a,
    src_out_ms: b,
    transition_in_ms: 0,
  });
  const plain = (clips: ReturnType<typeof clip>[]) => buildTimeMap(trim, [], [], clips, 10_000);

  it("is one span over the whole plan with no clips", () => {
    const m = fixtureMap();
    expect(framePlan(m, 10)).toHaveLength(85);
    expect(clipSpans(m, 10)).toEqual([{ clip: 0, planStart: 0, planLen: 85, firstK: 5 }]);
  });

  it("makes a split in order lossless", () => {
    const whole = framePlan(plain([]), 10);
    expect(whole).toHaveLength(86);
    expect(framePlan(plain([clip("cl0", 500, 4000), clip("cl1", 4000, 9000)]), 10)).toEqual(whole);
    expect(framePlan(plain([clip("cl0", 500, 4050), clip("cl1", 4050, 9000)]), 10)).toEqual(whole);
    expect(clipSpans(plain([clip("cl0", 500, 4000), clip("cl1", 4000, 9000)]), 10)).toEqual([
      { clip: 0, planStart: 0, planLen: 35, firstK: 5 },
      { clip: 1, planStart: 35, planLen: 51, firstK: 40 },
    ]);
  });

  it("keeps the frame count across a reorder", () => {
    const m = plain([clip("cl0", 4000, 9000), clip("cl1", 500, 4000)]);
    expect(framePlan(m, 10)).toHaveLength(86);
    expect(clipSpans(m, 10)).toEqual([
      { clip: 0, planStart: 0, planLen: 50, firstK: 40 },
      { clip: 1, planStart: 50, planLen: 36, firstK: 5 },
    ]);
  });

  it("lines the clips fixture's spans up with its seventy entry plan", () => {
    const m = clipsFixtureMap();
    expect(framePlan(m, 10)).toHaveLength(70);
    expect(clipSpans(m, 10)).toEqual([
      { clip: 0, planStart: 0, planLen: 49, firstK: 60 },
      { clip: 1, planStart: 49, planLen: 21, firstK: 5 },
    ]);
  });
});
