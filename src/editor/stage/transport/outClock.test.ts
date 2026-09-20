import { describe, expect, it } from "vitest";
import { buildTimeMap } from "../../../shared/math/remap";
import { clipsFixtureMap, fixtureMap } from "../../../shared/math/remap.fixture";
import { clipTick, needsOutClock } from "./outClock";

const clip = (id: string, a: number, b: number) => ({
  id,
  src_in_ms: a,
  src_out_ms: b,
  transition_in_ms: 0,
});

describe("needsOutClock", () => {
  it("is false for a document nobody has split", () => {
    expect(needsOutClock(fixtureMap())).toBe(false);
  });

  it("is false for a single clip, however it was trimmed", () => {
    const m = buildTimeMap({ in_ms: 500, out_ms: 9000 }, [], [], [clip("cl0", 2000, 6000)], 10_000);
    expect(needsOutClock(m)).toBe(false);
  });

  it("is true as soon as a second clip contributes frames", () => {
    expect(needsOutClock(clipsFixtureMap())).toBe(true);
  });

  it("is false when every clip but one is empty", () => {
    const m = buildTimeMap(
      { in_ms: 0, out_ms: 0 },
      [],
      [],
      [clip("cl0", 6000, 2000), clip("cl1", 500, 4000)],
      10_000,
    );
    expect(needsOutClock(m)).toBe(false);
  });
});

describe("clipTick", () => {
  const m = clipsFixtureMap();

  it("follows the element inside the segment the clock is in", () => {
    expect(clipTick(m, 0, 6000)).toEqual({ tOut: 0, t: 6000, seekTo: null, ended: false });
    expect(clipTick(m, 0, 7000)).toEqual({ tOut: 2000, t: 7000, seekTo: null, ended: false });
    expect(clipTick(m, 5250, 750)).toEqual({ tOut: 5250, t: 750, seekTo: null, ended: false });
    expect(clipTick(m, 5250, 766)).toEqual({ tOut: 5266, t: 766, seekTo: null, ended: false });
  });

  it("gives an element that landed a hair early one frame of slack", () => {
    expect(clipTick(m, 5000, 490)).toEqual({ tOut: 5000, t: 500, seekTo: null, ended: false });
    expect(clipTick(m, 5000, 480)).toEqual({ tOut: 5000, t: 500, seekTo: 500, ended: false });
  });

  it("crosses a contiguous join without a seek", () => {
    expect(clipTick(m, 3990, 8004)).toEqual({ tOut: 4004, t: 8004, seekTo: null, ended: false });
    expect(clipTick(m, 5990, 2510)).toEqual({ tOut: 6005, t: 2510, seekTo: null, ended: false });
  });

  it("seeks at a non contiguous join, to the next segment in OUTPUT order", () => {
    expect(clipTick(m, 4995, 9003)).toEqual({ tOut: 5000, t: 500, seekTo: 500, ended: false });
    expect(clipTick(m, 5495, 1002)).toEqual({ tOut: 5500, t: 2000, seekTo: 2000, ended: false });
  });

  it("does not re-seek while a slow seek is still landing", () => {
    let out = 5000;
    for (let i = 0; i < 5; i++) {
      const step = clipTick(m, out, 500);
      expect(step, `tick ${i}`).toEqual({ tOut: 5000, t: 500, seekTo: null, ended: false });
      out = step.tOut;
    }
  });

  it("ends the output when the element runs off the last segment", () => {
    expect(clipTick(m, 6990, 4001)).toEqual({ tOut: 7000, t: 3999, seekTo: null, ended: true });
    expect(clipTick(m, 6990, 3995)).toEqual({ tOut: 6995, t: 3995, seekTo: null, ended: false });
  });

  it("puts a stray element where the clock is", () => {
    expect(clipTick(m, 5250, 8900)).toEqual({ tOut: 5250, t: 750, seekTo: 750, ended: false });
    expect(clipTick(m, 0, 750)).toEqual({ tOut: 0, t: 6000, seekTo: 6000, ended: false });
  });

  it("clamps the clock to the output at both ends", () => {
    expect(clipTick(m, -40, 6000)).toEqual({ tOut: 0, t: 6000, seekTo: null, ended: false });
    expect(clipTick(m, 7500, 4001)).toEqual({ tOut: 7000, t: 3999, seekTo: null, ended: true });
  });

  it("steps over a contiguous sliver the element has already passed", () => {
    const sliver = buildTimeMap(
      { in_ms: 0, out_ms: 0 },
      [],
      [{ id: "s", start_ms: 1000, end_ms: 1010, factor: 2 }],
      [clip("cl0", 0, 5000), clip("cl1", 5000, 10_000)],
      10_000,
    );
    expect(sliver.segments.map((s) => [s.clipStart, s.clipEnd, s.factor, s.outStart])).toEqual([
      [0, 1000, 1, 0],
      [1000, 1010, 2, 1000],
      [1010, 5000, 1, 1005],
      [5000, 10_000, 1, 4995],
    ]);
    expect(clipTick(sliver, 995, 1015)).toEqual({ tOut: 1010, t: 1015, seekTo: null, ended: false });
  });

  it("is already over when the clip list shows nothing", () => {
    const empty = buildTimeMap({ in_ms: 0, out_ms: 0 }, [], [], [clip("cl0", 4000, 1000)], 10_000);
    expect(empty.segments).toEqual([]);
    expect(clipTick(empty, 0, 0)).toEqual({ tOut: 0, t: 0, seekTo: null, ended: true });
  });
});
