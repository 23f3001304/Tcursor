import { describe, it, expect } from "vitest";
import { pastDragThreshold, DRAG_THRESHOLD_PX } from "./dragThreshold";

describe("pastDragThreshold", () => {
  it("is false for zero movement - a bare click", () => {
    expect(pastDragThreshold(0, 0)).toBe(false);
  });

  it("is false for a sub-threshold jiggle on either axis", () => {
    expect(pastDragThreshold(2, 0)).toBe(false);
    expect(pastDragThreshold(0, -2)).toBe(false);
  });

  it("is true right at the default threshold", () => {
    expect(pastDragThreshold(DRAG_THRESHOLD_PX, 0)).toBe(true);
  });

  it("is true past the default threshold", () => {
    expect(pastDragThreshold(10, 0)).toBe(true);
    expect(pastDragThreshold(0, -10)).toBe(true);
  });

  it("uses Euclidean distance, not per-axis - a diagonal can cross the threshold even when neither axis alone does", () => {
    expect(pastDragThreshold(2, 2)).toBe(false);
    expect(pastDragThreshold(3, 3)).toBe(true);
  });

  it("honors a custom threshold (e.g. CamDragHandle's 4px)", () => {
    expect(pastDragThreshold(3, 0, 4)).toBe(false);
    expect(pastDragThreshold(4, 0, 4)).toBe(true);
  });

  it("ignores sign - movement in either direction counts the same", () => {
    expect(pastDragThreshold(-5, 0)).toBe(true);
    expect(pastDragThreshold(0, -5)).toBe(true);
  });
});
