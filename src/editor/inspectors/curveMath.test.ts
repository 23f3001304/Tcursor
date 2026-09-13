import { describe, it, expect } from "vitest";
import {
  clampHandle, clientToCurve, CURVE_SEEDS, curveOf, curvePath, handlePct, nudgeHandle, setHandle,
  springPathOf, toSvg, VIEW, Y_MAX, Y_MIN,
} from "./curveMath";
import { evalCubic } from "../../lib/cubicBezier";
import { spring } from "../../lib/spring";
import { ease } from "../timeline/layoutTrack";

const RECT = { left: 20, top: 10, width: 200, height: 320 };

describe("curve/SVG coordinate mapping", () => {
  it("maps the curve's own endpoints to the viewBox corners", () => {
    expect(toSvg(0, 0)).toEqual([0, 100]);
    expect(toSvg(1, 1)).toEqual([100, 0]);
  });

  it("derives its clamp band from the viewBox, not a hardcoded number", () => {
    expect(Y_MAX).toBeCloseTo(1 - VIEW.minY / 100, 6);
    expect(Y_MIN).toBeCloseTo(1 - (VIEW.minY + VIEW.h) / 100, 6);
  });

  it("inverts a pointer position back to the curve point it sits on", () => {
    expect(clientToCurve(RECT.left + RECT.width / 2, RECT.top + RECT.height / 2, RECT))
      .toEqual([0.5, (Y_MIN + Y_MAX) / 2]);
    for (const [x, y] of [[0.25, 0.9], [0.8, 0.1], [0.42, 1.2]] as [number, number][]) {
      const [sx, sy] = toSvg(x, y);
      const cx = RECT.left + ((sx - VIEW.minX) / VIEW.w) * RECT.width;
      const cy = RECT.top + ((sy - VIEW.minY) / VIEW.h) * RECT.height;
      expect(clientToCurve(cx, cy, RECT)[0]).toBeCloseTo(x, 3);
      expect(clientToCurve(cx, cy, RECT)[1]).toBeCloseTo(y, 3);
    }
  });

  it("clamps x to 0..1 and y to the visible band", () => {
    expect(clampHandle(-4, 9)).toEqual([0, Y_MAX]);
    expect(clampHandle(7, -9)).toEqual([1, Y_MIN]);
    expect(clientToCurve(RECT.left - 500, RECT.top - 500, RECT)).toEqual([0, Y_MAX]);
  });

  it("places a handle dot at the same percentage clientToCurve would read back", () => {
    // The dots are HTML, laid over the stretched SVG - their % has to be the exact inverse of the
    // pointer mapping, or a grabbed handle would jump on the first pointermove.
    const c = [0.25, 0.9, 0.8, 0.1] as const;
    for (const h of [0, 1] as const) {
      const [left, top] = handlePct([...c], h);
      const back = clientToCurve(RECT.left + (left / 100) * RECT.width, RECT.top + (top / 100) * RECT.height, RECT);
      expect(back[0]).toBeCloseTo(c[h * 2], 3);
      expect(back[1]).toBeCloseTo(c[h * 2 + 1], 3);
    }
  });
});

describe("handle editing", () => {
  it("replaces only the addressed control point", () => {
    const c = [0.1, 0.2, 0.3, 0.4] as const;
    expect(setHandle([...c], 0, 0.9, 0.8)).toEqual([0.9, 0.8, 0.3, 0.4]);
    expect(setHandle([...c], 1, 0.9, 0.8)).toEqual([0.1, 0.2, 0.9, 0.8]);
  });

  it("nudges by 0.05 per arrow press and stays clamped", () => {
    expect(nudgeHandle([0.5, 0.5, 0.5, 0.5], 0, 1, 0)).toEqual([0.55, 0.5, 0.5, 0.5]);
    expect(nudgeHandle([0.5, 0.5, 0.5, 0.5], 1, 0, -1)).toEqual([0.5, 0.5, 0.5, 0.45]);
    expect(nudgeHandle([1, 0.5, 0.5, 0.5], 0, 1, 0)[0]).toBe(1);
  });

  it("renders a path through both control points", () => {
    expect(curvePath([0.25, 0.1, 0.25, 1])).toBe("M 0 100 C 25 90, 25 0, 100 0");
  });
});

describe("curveOf", () => {
  it("parses a custom value straight back to its handles", () => {
    expect(curveOf("cubic(0.250,0.100,0.250,1.000)")).toEqual([0.25, 0.1, 0.25, 1]);
  });

  it("seeds a named preset from its own control points, and anything unknown from Smooth", () => {
    expect(curveOf("ease_out")).toEqual(CURVE_SEEDS.ease_out);
    expect(curveOf("wobble")).toEqual(CURVE_SEEDS.smooth);
  });
});

describe("preset seeds are honest", () => {
  it("reproduces linear / ease_in / ease_out / smooth EXACTLY as cubics", () => {
    for (const key of ["linear", "ease_in", "ease_out", "smooth"]) {
      const c = CURVE_SEEDS[key];
      for (const p of [0.1, 0.25, 0.5, 0.75, 0.9]) {
        expect(evalCubic(c[0], c[1], c[2], c[3], p)).toBeCloseTo(ease(key, p), 4);
      }
    }
  });

  it("keeps every preset seed inside the editor's own drag band", () => {
    for (const [key, c] of Object.entries(CURVE_SEEDS)) {
      for (const x of [c[0], c[2]]) { expect(x, key).toBeGreaterThanOrEqual(0); expect(x, key).toBeLessThanOrEqual(1); }
      for (const y of [c[1], c[3]]) { expect(y, key).toBeGreaterThanOrEqual(Y_MIN); expect(y, key).toBeLessThanOrEqual(Y_MAX); }
    }
  });
});

describe("springPathOf", () => {
  it("is null for everything that is not a spring", () => {
    expect(springPathOf("smooth")).toBeNull();
    expect(springPathOf("cubic(0.1,0.2,0.3,0.4)")).toBeNull();
    expect(springPathOf(null)).toBeNull();
  });

  it("samples the SAME oscillator the export evaluates, so the canvas cannot lie", () => {
    const d = springPathOf("spring(300.000,12.000,1.000)")!;
    const last = d.split(" L ").pop()!.split(" ").map(Number);
    expect(last[0]).toBeCloseTo(100, 6);                   // ends at progress 1
    expect(last[1]).toBeCloseTo(0, 6);                     // ...at value 1 (y is inverted)
    const mid = d.split(" L ")[24].split(" ").map(Number); // the 24th of 48 samples = p 0.5
    expect(1 - mid[1] / 100).toBeCloseTo(spring(300, 12, 1, 24 / 48), 2);
  });
});
