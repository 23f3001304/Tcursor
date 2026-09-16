import { describe, it, expect } from "vitest";
import { clampMaskRect, resizeMaskRect, snapMaskRect } from "./useMaskDrag";

type Rect = [number, number, number, number];
const R: Rect = [0.35, 0.4, 0.3, 0.2];

const near = (got: Rect, want: Rect) => {
  for (let i = 0; i < 4; i++) expect(got[i]).toBeCloseTo(want[i], 9);
};

describe("clampMaskRect mirrors the Rust clamp_rect", () => {
  it("keeps the rect on the canvas and at least one percent in each axis", () => {
    expect(clampMaskRect([-0.5, -0.5, 2, 2])).toEqual([0, 0, 1, 1]);
    expect(clampMaskRect([0.9, 0.9, 0.5, 0.5])).toEqual([0.5, 0.5, 0.5, 0.5]);
    expect(clampMaskRect([0.2, 0.2, 0.0001, 0.0001])).toEqual([0.2, 0.2, 0.01, 0.01]);
  });

  it("drops a non-finite rect rather than storing one", () => {
    expect(clampMaskRect([Number.NaN, 0, 0.2, 0.2])).toBeNull();
  });
});

describe("resizeMaskRect", () => {
  it("moves the whole rect without changing its size", () => {
    near(resizeMaskRect(R, "move", 0.1, -0.05), [0.45, 0.35, 0.3, 0.2]);
  });

  it("pulls one edge and leaves the opposite one put", () => {
    near(resizeMaskRect(R, "w", 0.05, 0), [0.4, 0.4, 0.25, 0.2]);
    near(resizeMaskRect(R, "e", 0.05, 0), [0.35, 0.4, 0.35, 0.2]);
    near(resizeMaskRect(R, "n", 0, 0.05), [0.35, 0.45, 0.3, 0.15]);
  });

  it("moves two edges from a corner", () => {
    near(resizeMaskRect(R, "se", 0.05, 0.05), [0.35, 0.4, 0.35, 0.25]);
  });
});

describe("snapMaskRect", () => {
  it("snaps an edge to the frame edge and reports the guide", () => {
    const s = snapMaskRect([0.004, 0.4, 0.3, 0.2], 0.01);
    expect(s.rect[0]).toBe(0);
    expect(s.guideX).toBe(0);
  });

  it("reports a guide on an axis that is already aligned without moving it", () => {
    const s = snapMaskRect([0.004, 0.4, 0.3, 0.2], 0.01);
    expect(s.rect[1]).toBe(0.4);
    expect(s.guideY).toBe(0.5);
  });

  it("snaps the centre to the middle of the frame", () => {
    const s = snapMaskRect([0.353, 0.4, 0.3, 0.2], 0.01);
    expect(s.rect[0]).toBeCloseTo(0.35, 6);
    expect(s.guideX).toBe(0.5);
  });

  it("leaves a rect that is near nothing alone", () => {
    const s = snapMaskRect([0.2, 0.3, 0.17, 0.11], 0.01);
    expect(s.rect).toEqual([0.2, 0.3, 0.17, 0.11]);
    expect([s.guideX, s.guideY]).toEqual([null, null]);
  });
});
