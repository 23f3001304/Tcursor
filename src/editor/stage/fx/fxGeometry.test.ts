// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { fxCamRect, fxFrameGeometry } from "./fxGeometry";
import type { PreviewLayout } from "../../../shared/ipc";

const layout = (over: Partial<PreviewLayout> = {}): PreviewLayout => ({
  screen: [0.1, 0.1, 0.8, 0.8],
  radius: 0,
  cam: null,
  canvas: [1920, 1080],
  ...over,
});
const noZoom = { cx: 0.5, cy: 0.5, scale: 1 };

describe("fxFrameGeometry mapCanvas", () => {
  it("maps a canvas point through the active span's crop rect", () => {
    const g = fxFrameGeometry(400, 400, layout({ src: [0.05, 0, 0.9, 1] }), noZoom, 1);
    const near = (a: [number, number] | null, b: [number, number] | null) => {
      expect(a![0]).toBeCloseTo(b![0], 9);
      expect(a![1]).toBeCloseTo(b![1], 9);
    };
    near(g.mapCanvas(0.05, 0), g.map(0, 0));
    near(g.mapCanvas(0.95, 1), g.map(1, 1));
    near(g.mapCanvas(0.5, 0.5), g.map(0.5, 0.5));
    expect(g.mapCanvas(0.01, 0.5)![0]).toBeLessThan(g.map(0, 0)![0]);
  });
});

describe("fxFrameGeometry", () => {
  it("renders at fxScale and reports the screen panel's height fraction", () => {
    const g = fxFrameGeometry(800, 400, layout(), noZoom, 0.5);
    expect([g.fxW, g.fxH]).toEqual([400, 200]);
    expect(g.screenScale).toBeCloseTo(0.8, 6);
  });

  it("never collapses to a zero-sized frame", () => {
    const g = fxFrameGeometry(1, 1, layout(), noZoom, 0.01);
    expect([g.fxW, g.fxH]).toEqual([1, 1]);
  });

  it("maps panel-local 0..1 onto the screen panel's rect when unzoomed", () => {
    const g = fxFrameGeometry(400, 400, layout(), noZoom, 1);
    expect(g.map(0, 0)).toEqual([40, 40]);
    expect(g.map(1, 1)).toEqual([360, 360]);
    expect(g.map(0.5, 0.5)).toEqual([200, 200]);
    expect(g.mapCanvas(0.25, 0.75)).toEqual(g.map(0.25, 0.75));
  });

  it("projects through the zoom crop, magnifying around the camera centre", () => {
    const g = fxFrameGeometry(400, 400, layout(), { cx: 0.5, cy: 0.5, scale: 2 }, 1);
    expect(g.map(0.5, 0.5)).toEqual([200, 200]);
    const [x] = g.map(0.75, 0.5)!;
    const [xUnzoomed] = fxFrameGeometry(400, 400, layout(), noZoom, 1).map(0.75, 0.5)!;
    expect(x - 200).toBeCloseTo((xUnzoomed - 200) * 2, 6);
  });

  it("clamps the crop inside the frame instead of sampling off its edge", () => {
    const g = fxFrameGeometry(400, 400, layout(), { cx: 0, cy: 0, scale: 2 }, 1);
    const [x, y] = g.map(0, 0)!;
    expect(x).toBeGreaterThanOrEqual(0);
    expect(y).toBeGreaterThanOrEqual(0);
  });

  it("falls back to a padded full frame before the layout has loaded", () => {
    const g = fxFrameGeometry(400, 400, null, noZoom, 1);
    const pad = 400 * 0.045;
    expect(g.map(0, 0)).toEqual([pad, pad]);
    expect(g.screenScale).toBeCloseTo((400 - 2 * pad) / 400, 6);
  });

  it("treats a degenerate zoom scale as no zoom rather than dividing by zero", () => {
    const g = fxFrameGeometry(400, 400, layout(), { cx: 0.5, cy: 0.5, scale: 0 }, 1);
    expect(Number.isFinite(g.map(0.5, 0.5)![0])).toBe(true);
  });
});

describe("fxCamRect", () => {
  const withCam = (camAlpha?: number) => layout({ cam: [0.6, 0.6, 0.3, 0.3, 0.05, 0, 0, 0, 0], camAlpha });

  it("is null when the layout has no camera panel", () => {
    expect(fxCamRect(layout(), 400, 400)).toBeNull();
    expect(fxCamRect(null, 400, 400)).toBeNull();
  });

  it("converts the layout fractions to FX px WITHOUT the zoom crop (the PiP is unzoomed)", () => {
    const r = fxCamRect(withCam(), 400, 400)!;
    r.rect.forEach((v, i) => expect(v).toBeCloseTo([240, 240, 360, 360][i], 6));
    expect(r.radius).toBeCloseTo(20, 6);
  });

  it("gates on the export's has_hole threshold, not layoutAt's lower draw threshold", () => {
    // A cross-fading panel between the two thresholds must NOT cut a hole the export never cuts.
    expect(fxCamRect(withCam(0.05), 400, 400)).toBeNull();
    expect(fxCamRect(withCam(0.2), 400, 400)).not.toBeNull();
  });

  it("treats a missing camAlpha as fully shown (the static, non-segmented layout)", () => {
    expect(fxCamRect(withCam(undefined), 400, 400)).not.toBeNull();
  });
});
