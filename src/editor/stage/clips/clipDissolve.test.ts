import { describe, it, expect } from "vitest";
import { buildTimeMap, clipOf } from "../../../shared/math/remap";
import { clipsFixtureMap, fixtureMap } from "../../../shared/math/remap.fixture";
import { clipDissolves, clipMixAt, preseekAt } from "./clipDissolve";

const clips = (t0: number, t1: number) => [
  { id: "cl1", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: t0 },
  { id: "cl0", src_in_ms: 500, src_out_ms: 4000, transition_in_ms: t1 },
];

const threeClips = (t1: number, t2: number) => [
  { id: "cl0", src_in_ms: 0, src_out_ms: 3000, transition_in_ms: 0 },
  { id: "cl1", src_in_ms: 3000, src_out_ms: 6000, transition_in_ms: t1 },
  { id: "cl2", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: t2 },
];

const threeClipMap = (t1: number, t2: number) =>
  buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], threeClips(t1, t2), 9000);

describe("clipDissolves (parity with export::render::clipmix)", () => {
  it("opens the window on the plan boundary, not on the segment's outStart", () => {
    expect(clipDissolves(clipsFixtureMap(), clips(0, 500), 10)).toEqual([
      { outStartMs: 4900, prevClip: 0, durMs: 500 },
    ]);
  });

  it("ignores the first clip's stored transition", () => {
    expect(clipDissolves(clipsFixtureMap(), clips(400, 0), 10)).toEqual([]);
  });

  it("has nothing to dissolve on a document with no clips", () => {
    expect(clipDissolves(fixtureMap(), [], 10)).toEqual([]);
  });

  it("clamps a transition longer than its own clip to the incoming span, the row Rust also pins", () => {
    expect(clipDissolves(clipsFixtureMap(), clips(0, 5000), 10)).toEqual([
      { outStartMs: 4900, prevClip: 0, durMs: 2100 },
    ]);
    expect(clipDissolves(clipsFixtureMap(), clips(0, 500), 10)[0].durMs).toBe(500);
  });
});

describe("clipMixAt", () => {
  const list = clipDissolves(clipsFixtureMap(), clips(0, 500), 10);

  it("is null outside the half open window", () => {
    expect(clipMixAt(list, 4899, "linear")).toBeNull();
    expect(clipMixAt(list, 5400, "linear")).toBeNull();
  });

  it("carries the incoming clip's weight on a linear curve, the one row Rust also pins", () => {
    expect(clipMixAt(list, 4900, "linear")).toEqual({ prevClip: 0, prevOutMs: 4899, alpha: 0 });
    expect(clipMixAt(list, 5150, "linear")?.alpha).toBeCloseTo(0.5, 6);
    expect(clipMixAt(list, 5399, "linear")?.alpha).toBeCloseTo(0.998, 6);
  });

  it("carries the smooth row Rust pins too, the same curve on both sides of the boundary", () => {
    expect(clipMixAt(list, 5150, "smooth")?.alpha).toBeCloseTo(0.5, 6);
    expect(clipMixAt(list, 5025, "smooth")?.alpha).toBeCloseTo(0.15625, 6);
  });

  it("lets the second join answer for itself once the first window cannot outlive its clip", () => {
    const three = clipDissolves(threeClipMap(5000, 400), threeClips(5000, 400), 10);
    expect(three).toEqual([
      { outStartMs: 3000, prevClip: 0, durMs: 3000 },
      { outStartMs: 6000, prevClip: 1, durMs: 400 },
    ]);
    expect(clipMixAt(three, 6100, "linear")).toEqual({ prevClip: 1, prevOutMs: 5999, alpha: 0.25 });
  });
});

describe("preseekAt", () => {
  const map = clipsFixtureMap();
  const list = clipDissolves(map, clips(0, 500), 10);

  it("asks for the middle of the latched frame a second ahead of the boundary and through the window", () => {
    expect(preseekAt(list, map, 3900, 10)).toBe(8950);
    expect(preseekAt(list, map, 4899, 10)).toBe(8950);
    expect(preseekAt(list, map, 5399, 10)).toBe(8950);
  });

  it("holds the frame the export latched, not the source frame before it, at the preview's own 60 fps", () => {
    const list60 = clipDissolves(map, clips(0, 500));
    expect(list60[0].outStartMs).toBe(4983);
    const ms = preseekAt(list60, map, 4982);
    expect(ms).toBeCloseTo(8991.67, 2);
    expect(Math.floor(((ms ?? 0) * 60) / 1000)).toBe(539);
    expect(Math.floor((clipOf(map, 4982) * 60) / 1000)).toBe(538);
  });

  it("asks for nothing when no boundary is near", () => {
    expect(preseekAt(list, map, 3899, 10)).toBeNull();
    expect(preseekAt(list, map, 1000, 10)).toBeNull();
    expect(preseekAt(list, map, 5400, 10)).toBeNull();
  });

  it("asks for nothing at all when no clip dissolves", () => {
    expect(preseekAt([], map, 4800)).toBeNull();
  });
});
