// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { CAM_SNAP_MS, snapKeyframeMs } from "./camSnap";

describe("snapKeyframeMs", () => {
  it("snaps to a layout-segment edge just inside the window", () => {
    expect(snapKeyframeMs(1930, [1000, 2000], [])).toBe(2000);
    expect(snapKeyframeMs(2070, [1000, 2000], [])).toBe(2000);
  });

  it("snaps at exactly +/- CAM_SNAP_MS (the window is inclusive)", () => {
    expect(CAM_SNAP_MS).toBe(80);
    expect(snapKeyframeMs(1920, [2000], [])).toBe(2000);
    expect(snapKeyframeMs(2080, [2000], [])).toBe(2000);
  });

  it("leaves the time alone one millisecond outside the window", () => {
    expect(snapKeyframeMs(1919, [2000], [])).toBe(1919);
    expect(snapKeyframeMs(2081, [2000], [])).toBe(2081);
  });

  it("snaps to another keyframe's time, not just segment edges", () => {
    expect(snapKeyframeMs(3040, [], [3000, 9000])).toBe(3000);
  });

  it("picks the NEAREST candidate when several are in range", () => {
    expect(snapKeyframeMs(2050, [2000], [2060])).toBe(2060);
    expect(snapKeyframeMs(2010, [2000], [2060])).toBe(2000);
  });

  it("returns the time unchanged with no candidates at all", () => {
    expect(snapKeyframeMs(1234, [], [])).toBe(1234);
  });

  it("does not snap to far-away candidates on either side", () => {
    expect(snapKeyframeMs(5000, [0, 1000, 20_000], [400, 30_000])).toBe(5000);
  });
});
