import { describe, it, expect } from "vitest";
import { clipsFixtureMap } from "../../../shared/math/remap.fixture";
import { clipExtraStyle, clipLabel, clipRegions, dropIndex } from "./clipModel";

const clips = [
  { id: "cl1", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: 0 },
  { id: "cl0", src_in_ms: 500, src_out_ms: 4000, transition_in_ms: 500 },
];

describe("clipRegions", () => {
  it("puts each pill on its source range and numbers it by output position", () => {
    const got = clipRegions(clips, clipsFixtureMap()).map((c) => [c.id, c.start_ms, c.end_ms, c.order]);
    expect(got).toEqual([
      ["cl0", 500, 4000, 2],
      ["cl1", 6000, 9000, 1],
    ]);
  });

  it("carries each clip's OUTPUT length, which is what the export will spend on it", () => {
    const by = Object.fromEntries(clipRegions(clips, clipsFixtureMap()).map((c) => [c.id, c.outMs]));
    expect(by).toEqual({ cl1: 5000, cl0: 2000 });
  });

  it("gives overlapping pills their own rows", () => {
    const over = [
      { id: "cl0", src_in_ms: 0, src_out_ms: 5000, transition_in_ms: 0 },
      { id: "cl1", src_in_ms: 2000, src_out_ms: 7000, transition_in_ms: 0 },
    ];
    const rows = clipRegions(over, clipsFixtureMap()).map((c) => c.layer);
    expect(new Set(rows).size).toBe(2);
  });

  it("is empty for a document nobody has split", () => {
    expect(clipRegions([], clipsFixtureMap())).toEqual([]);
  });
});

describe("clipLabel", () => {
  it("is the output position and the output length", () => {
    const [a, b] = clipRegions(clips, clipsFixtureMap());
    expect(clipLabel(a)).toBe("2 - 2.0s");
    expect(clipLabel(b)).toBe("1 - 5.0s");
  });
});

describe("clipExtraStyle", () => {
  it("ramps the left edge by the transition's share of the pill", () => {
    const [a, b] = clipRegions(clips, clipsFixtureMap());
    expect(clipExtraStyle(a, a.start_ms, a.end_ms)).toEqual({ "--fin": "14.285714285714285%" });
    expect(clipExtraStyle(b, b.start_ms, b.end_ms)).toEqual({ "--fin": "0%" });
  });
});

describe("dropIndex", () => {
  it("names the index of the clip the pointer is over", () => {
    expect(dropIndex(clips, "cl0", 7000)).toBe(0);
    expect(dropIndex(clips, "cl1", 1000)).toBe(1);
  });

  it("is null over empty space or over the clip being dragged", () => {
    expect(dropIndex(clips, "cl0", 5000)).toBeNull();
    expect(dropIndex(clips, "cl0", 1000)).toBeNull();
    expect(dropIndex(clips, "nope", 7000)).toBeNull();
  });
});
