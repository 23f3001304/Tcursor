import { describe, it, expect } from "vitest";
import type { PanelPose } from "../../../shared/edit";
import {
  movedPose,
  poseOfRect,
  rectAspect,
  resizedPose,
  MAX_SIZE,
  MIN_SIZE,
  SNAP_PX,
  SNAP_TARGETS,
  type Rect,
  type SnapTol,
} from "./arrangeMath";

const OW = 1920,
  OH = 1080;

const TOL: SnapTol = [SNAP_PX / OW, SNAP_PX / OH];

const RECT: Rect = [0.25, 0.25, 0.5, 0.5];
const ASPECT = rectAspect(RECT, OW, OH);
const CAM: Rect = [0.7, 0.7, 0.2, 0.2];
const pose = (cx: number, cy: number, size: number): PanelPose => ({ cx, cy, size });

describe("poseOfRect / rectAspect", () => {
  it("reads a rect's centre and height, and its PIXEL aspect (w/h are fractions of different axes)", () => {
    expect(poseOfRect(RECT)).toEqual({ cx: 0.5, cy: 0.5, size: 0.5 });
    expect(ASPECT).toBeCloseTo(16 / 9, 9);
    expect(rectAspect(CAM, OW, OH)).toBeCloseTo(16 / 9, 9);
  });
});

describe("movedPose", () => {
  it("translates the centre by the pointer delta and leaves the size alone", () => {
    const out = movedPose(pose(0.5, 0.5, 0.5), [0.1, -0.08], ASPECT, OW, OH, null);
    expect(out.pose).toEqual({ cx: 0.6, cy: 0.42, size: 0.5 });
    expect([out.guideX, out.guideY]).toEqual([null, null]);
  });

  it("clamps the centre into the frame (the same 0..1 range the op clamps to)", () => {
    const out = movedPose(pose(0.9, 0.1, 0.3), [0.5, -0.5], ASPECT, OW, OH, null);
    expect(out.pose.cx).toBe(1);
    expect(out.pose.cy).toBe(0);
  });

  it("snaps the CENTRE onto the frame centre and reports the guide", () => {
    const out = movedPose(pose(0.5, 0.5, 0.3), [0.003, 0], ASPECT, OW, OH, TOL);
    expect(out.pose.cx).toBeCloseTo(0.5, 9);
    expect(out.guideX).toBe(0.5);
  });

  it("snaps an EDGE, not just the centre - the left edge lands on the 4.5% safe margin", () => {
    const w = (0.2 * OH * ASPECT) / OW;
    const start = pose(0.045 + w / 2 + 3 / OW, 0.5, 0.2);
    const out = movedPose(start, [0, 0], ASPECT, OW, OH, TOL);
    expect(out.pose.cx - w / 2).toBeCloseTo(0.045, 9);
    expect(out.guideX).toBe(0.045);
  });

  it("snaps to a third on the vertical axis", () => {
    const out = movedPose(pose(0.5, 1 / 3 + 0.005, 0.2), [0, 0], ASPECT, OW, OH, TOL);
    expect(out.pose.cy).toBeCloseTo(1 / 3, 9);
    expect(out.guideY).toBe(1 / 3);
  });

  it("does NOT snap past the 8 stage-px threshold", () => {
    const off = 9 / OW;
    const out = movedPose(pose(0.5 + off, 0.5, 0.2), [0, 0], ASPECT, OW, OH, TOL);
    expect(out.pose.cx).toBeCloseTo(0.5 + off, 9);
    expect(out.guideX).toBeNull();
  });

  it("Alt (snap=false) leaves an otherwise-snapping drag exactly where the pointer put it", () => {
    const start = pose(0.5, 0.5, 0.3);
    const on = movedPose(start, [0.003, 0.002], ASPECT, OW, OH, TOL);
    const off = movedPose(start, [0.003, 0.002], ASPECT, OW, OH, null);
    expect(on.pose.cx).not.toBeCloseTo(off.pose.cx, 9);
    expect(off.pose).toEqual({ cx: 0.503, cy: 0.502, size: 0.3 });
    expect([off.guideX, off.guideY]).toEqual([null, null]);
  });

  it("drops a guide the CLAMP pulled the panel off (guides re-derived from the final pose)", () => {
    const out = movedPose(pose(0.9, 0.5, 0.094), [0.2, 0], ASPECT, OW, OH, TOL);
    expect(out.pose.cx).toBe(1);
    expect(out.guideX).toBeNull();
  });

  it("offers exactly the authored target set on both axes", () => {
    expect(SNAP_TARGETS).toEqual([0.045, 1 / 3, 0.5, 2 / 3, 1 - 0.045]);
  });
});

describe("resizedPose", () => {
  const at = (corner: "tl" | "tr" | "bl" | "br", ptr: [number, number], snap: SnapTol | null = null) =>
    resizedPose(RECT, corner, ptr, ASPECT, OW, OH, snap);

  it("is the identity when the pointer is still on the corner it grabbed", () => {
    const out = at("br", [0.75, 0.75]);
    expect(out.pose.cx).toBeCloseTo(0.5, 9);
    expect(out.pose.cy).toBeCloseTo(0.5, 9);
    expect(out.pose.size).toBeCloseTo(0.5, 9);
  });

  it("anchors the OPPOSITE corner: dragging bottom-right keeps the top-left pinned", () => {
    const out = at("br", [1, 1]);
    const w = (out.pose.size * OH * ASPECT) / OW;
    expect(out.pose.size).toBeCloseTo(0.75, 9);
    expect(out.pose.cx - w / 2).toBeCloseTo(0.25, 9);
    expect(out.pose.cy - out.pose.size / 2).toBeCloseTo(0.25, 9);
  });

  it("anchors the opposite corner for every corner - dragging top-left pins bottom-right", () => {
    const out = at("tl", [0.5, 0.5]);
    const w = (out.pose.size * OH * ASPECT) / OW;
    expect(out.pose.cx + w / 2).toBeCloseTo(0.75, 9);
    expect(out.pose.cy + out.pose.size / 2).toBeCloseTo(0.75, 9);
  });

  it("never stretches: width stays height x the panel's own aspect for an off-diagonal drag", () => {
    const out = at("tr", [0.95, 0.4]);
    const wPx = out.pose.size * OH * ASPECT,
      hPx = out.pose.size * OH;
    expect(wPx / hPx).toBeCloseTo(ASPECT, 9);
  });

  it("clamps the size to the op's own range in both directions", () => {
    expect(at("br", [9, 9]).pose.size).toBe(MAX_SIZE);
    expect(at("br", [0.25, 0.25]).pose.size).toBe(MIN_SIZE);
    expect(at("br", [-5, -5]).pose.size).toBe(MIN_SIZE);
  });

  it("snaps the DRAGGED edge onto a target (the anchor stays put) and reports the guide", () => {
    const corner = 0.25 + 455 / OH;
    const out = at("br", [corner, corner], TOL);
    expect(out.pose.size).toBeCloseTo(450 / OH, 9);
    expect(out.guideY).toBe(2 / 3);
    expect(out.pose.cy - out.pose.size / 2).toBeCloseTo(0.25, 9);
  });

  it("Alt (snap=false) keeps the same drag off the target", () => {
    const corner = 0.25 + 455 / OH;
    const out = at("br", [corner, corner], null);
    expect(out.pose.size).toBeCloseTo(455 / OH, 9);
    expect([out.guideX, out.guideY]).toEqual([null, null]);
  });
});
