import { describe, it, expect } from "vitest";
import { gifIndexAt, frameEnds } from "./gifFrames";

describe("frameEnds", () => {
  it("turns per-frame durations into cumulative end times", () => {
    expect(frameEnds([100, 150, 150])).toEqual([100, 250, 400]);
    expect(frameEnds([])).toEqual([]);
  });

  it("gives a frame with no stated duration the GIF default rather than zero", () => {
    expect(frameEnds([0, 0])).toEqual([100, 200]);
  });
});

describe("gifIndexAt", () => {
  it("picks the frame whose span contains the wrapped playhead", () => {
    const ends = [100, 250, 400];
    expect(gifIndexAt(ends, 0)).toBe(0);
    expect(gifIndexAt(ends, 99)).toBe(0);
    expect(gifIndexAt(ends, 100)).toBe(1);
    expect(gifIndexAt(ends, 249)).toBe(1);
    expect(gifIndexAt(ends, 250)).toBe(2);
    expect(gifIndexAt(ends, 399)).toBe(2);
  });

  it("wraps at the end of the loop and clamps a playhead before zero", () => {
    const ends = [100, 250, 400];
    expect(gifIndexAt(ends, 400)).toBe(0);
    expect(gifIndexAt(ends, 850)).toBe(0);
    expect(gifIndexAt(ends, 950)).toBe(1);
    expect(gifIndexAt(ends, -10)).toBe(0);
  });

  it("is 0 when nothing has been decoded, so the draw is never out of range", () => {
    expect(gifIndexAt([], 10)).toBe(0);
  });
});
