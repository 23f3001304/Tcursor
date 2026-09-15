// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { ease } from "../timeline/model/layoutTrack";
import { buildGraph, GRAPH_H, GRAPH_W, type GraphInput } from "./graphModel";
import { msToX, progressToY, SAMPLES_PER_RAMP, xToMs, yToProgress } from "./graphCoords";

const pts = (d: string): [number, number][] =>
  d
    .replace(/^M\s*/, "")
    .split(" L ")
    .map((s) => {
      const [x, y] = s.trim().split(/\s+/).map(Number);
      return [x, y] as [number, number];
    });

const ZOOM: GraphInput = {
  lane: "zoom",
  startMs: 0,
  endMs: 3000,
  rampIn: { easing: "smooth", durMs: 450 },
  rampOut: { easing: "ease_out", durMs: 700 },
  peak: 2,
};

describe("buildGraph windows", () => {
  it("gives each ramp the x window its duration asks for", () => {
    const m = buildGraph(ZOOM);
    const [rin, rout] = m.ramps;
    expect(rin.which).toBe("in");
    expect(rout.which).toBe("out");
    expect(rin.x0).toBeCloseTo(msToX(m, ZOOM, 0), 2);
    expect(rin.x1).toBeCloseTo(msToX(m, ZOOM, 450), 2);
    expect(rout.x0).toBeCloseTo(msToX(m, ZOOM, 2300), 2);
    expect(rout.x1).toBeCloseTo(msToX(m, ZOOM, 3000), 2);
    expect(m.plateau).not.toBeNull();
    expect(m.plateau!.x0).toBeCloseTo(rin.x1, 2);
    expect(m.plateau!.x1).toBeCloseTo(rout.x0, 2);
  });

  it("puts the whole ramp in the plot and leaves the gutters to the neighbours", () => {
    const m = buildGraph(ZOOM);
    expect(m.width).toBe(GRAPH_W);
    expect(m.height).toBe(GRAPH_H);
    expect(m.ramps[0].x0).toBe(m.plot.x);
    expect(m.ramps[1].x1).toBeCloseTo(m.plot.x + m.plot.w, 2);
  });

  it("a camera move is one ramp over its whole span, with no hold", () => {
    const cam: GraphInput = {
      lane: "cam",
      startMs: 1000,
      endMs: 1350,
      rampIn: { easing: "smooth", durMs: 350 },
      rampOut: null,
      peak: 1,
    };
    const m = buildGraph(cam);
    expect(m.ramps).toHaveLength(1);
    expect(m.plateau).toBeNull();
    expect(m.ramps[0].x1).toBeCloseTo(m.plot.x + m.plot.w, 2);
    expect(m.yTicks.map((t) => t.label)).toEqual(["0%", "100%"]);
  });
});

describe("buildGraph sampling", () => {
  it("draws the in ramp through ease()", () => {
    const m = buildGraph(ZOOM);
    const p = pts(m.ramps[0].path);
    expect(p).toHaveLength(SAMPLES_PER_RAMP + 1);
    for (const i of [0, 32, 64]) {
      const at = i / SAMPLES_PER_RAMP;
      expect(p[i][1]).toBeCloseTo(progressToY(m, "in", ease("smooth", at)), 1);
    }
    expect(p[0][1]).toBeCloseTo(m.yTicks[0].y, 1);
    expect(p[64][1]).toBeCloseTo(m.yTicks[1].y, 1);
  });

  it("draws the out ramp backwards, from the peak to rest", () => {
    const m = buildGraph(ZOOM);
    const p = pts(m.ramps[1].path);
    for (const i of [0, 16, 48]) {
      const at = i / SAMPLES_PER_RAMP;
      expect(p[i][1]).toBeCloseTo(progressToY(m, "out", ease("ease_out", at)), 1);
    }
    expect(p[0][1]).toBeCloseTo(m.yTicks[1].y, 1);
    expect(p[SAMPLES_PER_RAMP][1]).toBeCloseTo(m.yTicks[0].y, 1);
  });

  it("gives room to an overshoot instead of clipping it", () => {
    const over = { ...ZOOM, rampIn: { easing: "keys(0 0 0 0 0.2 1.4 b,1 1 -0.3 0 0 0 b)", durMs: 450 } };
    const m = buildGraph(over);
    const ys = pts(m.ramps[0].path).map((q) => q[1]);
    expect(Math.min(...ys)).toBeGreaterThan(m.plot.y);
    expect(Math.min(...ys)).toBeLessThan(m.yTicks[1].y);
  });
});

describe("buildGraph keys and editability", () => {
  it("a keys curve gets its dots and its handles", () => {
    const k = { ...ZOOM, rampIn: { easing: "keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)", durMs: 450 } };
    const m = buildGraph(k);
    expect(m.ramps[0].editable).toBe(true);
    expect(m.ramps[0].keys).toHaveLength(2);
    const [a, b] = m.ramps[0].keys!;
    expect(a.x).toBeCloseTo(m.ramps[0].x0, 2);
    expect(b.x).toBeCloseTo(m.ramps[0].x1, 2);
    expect(a.outX - a.x).toBeCloseTo((m.ramps[0].x1 - m.ramps[0].x0) * 0.333, 1);
  });

  it("a named curve has no dots yet but is editable, a spring is neither", () => {
    const m = buildGraph(ZOOM);
    expect(m.ramps[0].keys).toBeNull();
    expect(m.ramps[0].editable).toBe(true);
    const spr = buildGraph({ ...ZOOM, rampIn: { easing: "spring(140,7,1)", durMs: 450 } });
    expect(spr.ramps[0].keys).toBeNull();
    expect(spr.ramps[0].editable).toBe(false);
  });
});

describe("buildGraph extras", () => {
  it("draws the follow hint only when asked", () => {
    expect(buildGraph(ZOOM).hint).toBeNull();
    expect(buildGraph({ ...ZOOM, followHint: true }).hint).not.toBeNull();
  });

  it("draws a ghost per neighbour, in the gutter outside the span", () => {
    expect(buildGraph(ZOOM).ghosts).toHaveLength(0);
    const m = buildGraph({
      ...ZOOM,
      prev: { endMs: -200, easingOut: "smooth", durMs: 400 },
      next: { startMs: 3200, easingIn: "smooth", durMs: 400 },
    });
    expect(m.ghosts).toHaveLength(2);
    const before = pts(m.ghosts[0]).map((q) => q[0]);
    const after = pts(m.ghosts[1]).map((q) => q[0]);
    expect(Math.max(...before)).toBeLessThanOrEqual(m.plot.x);
    expect(Math.min(...after)).toBeGreaterThanOrEqual(m.plot.x + m.plot.w);
    expect(Math.min(...before)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...after)).toBeLessThanOrEqual(m.width);
  });

  it("labels a few ms ticks across the span", () => {
    const m = buildGraph(ZOOM);
    expect(m.ticks.length).toBeGreaterThanOrEqual(3);
    expect(m.ticks.length).toBeLessThanOrEqual(6);
    expect(m.ticks[0].label).toBe("0ms");
    expect(m.yTicks.map((t) => t.label)).toEqual(["1.0x", "2.0x"]);
  });
});

describe("mappings", () => {
  it("msToX and xToMs round-trip", () => {
    const m = buildGraph(ZOOM);
    for (const ms of [0, 450, 1500, 2300, 3000]) {
      expect(xToMs(m, ZOOM, msToX(m, ZOOM, ms))).toBeCloseTo(ms, 6);
    }
    expect(msToX(m, ZOOM, 0)).toBeCloseTo(m.plot.x, 6);
  });

  it("progressToY and yToProgress round-trip, mirrored on the out ramp", () => {
    const m = buildGraph(ZOOM);
    for (const v of [0, 0.25, 1, 1.2]) {
      expect(yToProgress(m, "in", progressToY(m, "in", v))).toBeCloseTo(v, 6);
      expect(yToProgress(m, "out", progressToY(m, "out", v))).toBeCloseTo(v, 6);
    }
    expect(progressToY(m, "in", 1)).toBeCloseTo(m.yTicks[1].y, 6);
    expect(progressToY(m, "out", 1)).toBeCloseTo(m.yTicks[0].y, 6);
  });
});
