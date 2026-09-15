// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { mapCanvasClickToZoomTarget, mapZoomTargetToCanvasPoint, zoomTargetPoint } from "./zoomTargetMapper";
import type { PreviewLayout } from "../../../shared/ipc";

const CANVAS_W = 1280,
  CANVAS_H = 720;

const fakeCanvas = {
  width: CANVAS_W,
  height: CANVAS_H,
  getBoundingClientRect: () => ({ left: 40, top: 24, width: CANVAS_W / 2, height: CANVAS_H / 2 }),
};
const LAYOUT: PreviewLayout = {
  screen: [0.1, 0.05, 0.7, 0.8],
  radius: 0,
  cam: null,
  canvas: [CANVAS_W, CANVAS_H],
};

const clientOf = ([fx, fy]: [number, number]) => ({
  clientX: 40 + fx * (CANVAS_W / 2),
  clientY: 24 + fy * (CANVAS_H / 2),
});

describe("zoomTargetPoint", () => {
  it("returns the stored point for a Fixed (Region) target and null for cursor", () => {
    expect(zoomTargetPoint({ fixed: { x: 0.25, y: 0.75 } })).toEqual([0.25, 0.75]);
    expect(zoomTargetPoint("cursor")).toBeNull();
    expect(zoomTargetPoint(null)).toBeNull();
  });
});

describe("mapZoomTargetToCanvasPoint (T31 stage reticle)", () => {
  it("is the exact forward of mapCanvasClickToZoomTarget at scale 1", () => {
    const cam = { scale: 1, cx: 0.5, cy: 0.5 };
    for (const t of [
      [0, 0],
      [0.5, 0.5],
      [0.25, 0.8],
      [1, 1],
    ] as [number, number][]) {
      const pt = mapZoomTargetToCanvasPoint({
        tx: t[0],
        ty: t[1],
        canvasW: CANVAS_W,
        canvasH: CANVAS_H,
        layout: LAYOUT,
        cam,
      });
      const back = mapCanvasClickToZoomTarget({
        ...clientOf(pt),
        canvasElement: fakeCanvas,
        layout: LAYOUT,
        cam,
      });
      expect(back).not.toBeNull();
      expect(back![0]).toBeCloseTo(t[0], 6);
      expect(back![1]).toBeCloseTo(t[1], 6);
    }
  });

  it("round-trips through a live zoom crop (scale 2.5, off-centre camera)", () => {
    const cam = { scale: 2.5, cx: 0.4, cy: 0.6 };
    for (const t of [
      [0.4, 0.6],
      [0.35, 0.55],
      [0.5, 0.7],
    ] as [number, number][]) {
      const pt = mapZoomTargetToCanvasPoint({
        tx: t[0],
        ty: t[1],
        canvasW: CANVAS_W,
        canvasH: CANVAS_H,
        layout: LAYOUT,
        cam,
      });
      const back = mapCanvasClickToZoomTarget({
        ...clientOf(pt),
        canvasElement: fakeCanvas,
        layout: LAYOUT,
        cam,
      });
      expect(back).not.toBeNull();
      expect(back![0]).toBeCloseTo(t[0], 5);
      expect(back![1]).toBeCloseTo(t[1], 5);
    }
  });

  it("puts the camera's own aim point at the centre of the canvas when zoomed in", () => {
    const [fx, fy] = mapZoomTargetToCanvasPoint({
      tx: 0.5,
      ty: 0.5,
      canvasW: CANVAS_W,
      canvasH: CANVAS_H,
      layout: LAYOUT,
      cam: { scale: 2, cx: 0.5, cy: 0.5 },
    });
    expect(fx).toBeCloseTo(0.5, 6);
    expect(fy).toBeCloseTo(0.5, 6);
  });

  it("reports fractions outside 0..1 for an aim point cropped out of frame", () => {
    const [fx] = mapZoomTargetToCanvasPoint({
      tx: 0.02,
      ty: 0.5,
      canvasW: CANVAS_W,
      canvasH: CANVAS_H,
      layout: LAYOUT,
      cam: { scale: 3, cx: 0.9, cy: 0.5 },
    });
    expect(fx).toBeLessThan(0);
  });
});
