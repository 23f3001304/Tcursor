// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { MotionGraph } from "./MotionGraph";
import type { GraphInput } from "./graphModel";
import { canonicalKeys, evalKeys, keysToString, MAX_KEYS, parseKeys, type Key, type Keys } from "./keys";
import {
  addKeyAt,
  MIN_RAMP_MS,
  moveHandle,
  moveKey,
  nudgeKey,
  patchOf,
  removeKey,
  retimeIndex,
  retimeMs,
  toKeysInput,
  withCurve,
  type GraphPatch,
} from "./graphEdits";

const THREE = parseKeys("keys(0 0 0 0 0.2 0 b,0.5 0.5 -0.2 0 0.2 0 b,1 1 -0.2 0 0 0 b)")!;
const TWO = parseKeys("keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)")!;
const ts = (k: Keys) => k.keys.map((x) => x.t);

const many = (n: number): Keys =>
  canonicalKeys({
    keys: Array.from({ length: n }, (_, i): Key => ({
      t: i / (n - 1),
      v: i / (n - 1),
      in: [0, 0],
      out: [0, 0],
      mode: "b",
    })),
  });

const ZOOM: GraphInput = {
  lane: "zoom",
  startMs: 1000,
  endMs: 4000,
  rampIn: { easing: "smooth", durMs: 450 },
  rampOut: { easing: "smooth", durMs: 700 },
  peak: 2,
};

describe("moveKey", () => {
  it("holds an inner key strictly between its neighbours", () => {
    expect(ts(moveKey(THREE, 1, 1.4, 0.5))[1]).toBeCloseTo(0.999, 3);
    expect(ts(moveKey(THREE, 1, -0.4, 0.5))[1]).toBeCloseTo(0.001, 3);
    expect(ts(moveKey(THREE, 1, 0.3, 0.5))[1]).toBeCloseTo(0.3, 3);
  });

  it("pins the endpoints in time, whatever the pointer says", () => {
    expect(ts(moveKey(THREE, 0, 0.4, 0))).toEqual([0, 0.5, 1]);
    expect(ts(moveKey(THREE, 2, 0.4, 1))).toEqual([0, 0.5, 1]);
  });

  it("leaves the value free, so overshoot and anticipation survive", () => {
    expect(moveKey(THREE, 1, 0.5, 1.4).keys[1].v).toBeCloseTo(1.4, 3);
    expect(moveKey(THREE, 1, 0.5, -0.3).keys[1].v).toBeCloseTo(-0.3, 3);
  });
});

describe("moveHandle", () => {
  it("clamps out_dx to the next key and in_dx to the previous one", () => {
    expect(moveHandle(THREE, 0, "out", 0.9, 0.3).keys[0].out).toEqual([0.5, 0.3]);
    expect(moveHandle(THREE, 0, "out", -0.2, 0).keys[0].out).toEqual([0, 0]);
    expect(moveHandle(THREE, 2, "in", -0.9, 0).keys[2].in).toEqual([-0.5, 0]);
    expect(moveHandle(THREE, 2, "in", 0.3, 0).keys[2].in).toEqual([0, 0]);
  });

  it("never clamps dy - that is where the overshoot is", () => {
    expect(moveHandle(THREE, 0, "out", 0.2, 2.5).keys[0].out[1]).toBeCloseTo(2.5, 3);
  });
});

describe("addKeyAt", () => {
  it("inserts the new key on the curve, so the shape does not jump", () => {
    const next = addKeyAt(TWO, 0.4);
    expect(next.keys).toHaveLength(3);
    expect(next.keys[1].t).toBeCloseTo(0.4, 3);
    expect(next.keys[1].v).toBeCloseTo(evalKeys(TWO, 0.4), 3);
    expect(next.keys[1].mode).toBe("b");
  });

  it("inherits the mode of the segment it splits", () => {
    const held = parseKeys("keys(0 0 0 0 0 0 h,1 1 0 0 0 0 l)")!;
    expect(addKeyAt(held, 0.5).keys[1].mode).toBe("h");
  });

  it("refuses a ninth key and a key on top of one that is there", () => {
    const full = many(MAX_KEYS);
    expect(full.keys).toHaveLength(8);
    expect(addKeyAt(full, 0.45)).toBe(full);
    expect(addKeyAt(THREE, 0.5)).toBe(THREE);
    expect(addKeyAt(THREE, 0.5001)).toBe(THREE);
  });
});

describe("removeKey", () => {
  it("refuses the first and the last key", () => {
    expect(removeKey(THREE, 0)).toBe(THREE);
    expect(removeKey(THREE, 2)).toBe(THREE);
    expect(removeKey(TWO, 1)).toBe(TWO);
  });

  it("drops an inner key and leaves a curve that still parses", () => {
    const next = removeKey(THREE, 1);
    expect(ts(next)).toEqual([0, 1]);
    expect(parseKeys(keysToString(next))).not.toBeNull();
  });
});

describe("nudgeKey", () => {
  it("moves by 0.01, or 0.1 with Shift", () => {
    expect(nudgeKey(THREE, 1, 0, 1).keys[1].v).toBeCloseTo(0.51, 3);
    expect(nudgeKey(THREE, 1, 0, -1, true).keys[1].v).toBeCloseTo(0.4, 3);
    expect(nudgeKey(THREE, 1, 1, 0).keys[1].t).toBeCloseTo(0.51, 3);
    expect(nudgeKey(THREE, 1, -1, 0, true).keys[1].t).toBeCloseTo(0.4, 3);
  });

  it("nudging an endpoint moves its value only", () => {
    const next = nudgeKey(THREE, 2, 1, 1);
    expect(next.keys[2].t).toBe(1);
    expect(next.keys[2].v).toBeCloseTo(1.01, 3);
  });
});

describe("retiming", () => {
  it("retimes from the key that meets the hold", () => {
    expect(retimeIndex("in", 3)).toBe(2);
    expect(retimeIndex("out", 3)).toBe(0);
  });

  it("writes whole milliseconds, clamped to the region", () => {
    expect(retimeMs(ZOOM, "in", 1620.4)).toBe(620);
    expect(retimeMs(ZOOM, "out", 3100.6)).toBe(899);
    expect(retimeMs(ZOOM, "in", 500)).toBe(MIN_RAMP_MS);
    expect(retimeMs(ZOOM, "out", 9000)).toBe(MIN_RAMP_MS);
    expect(retimeMs(ZOOM, "in", 99000)).toBe(3000);
  });
});

describe("commits", () => {
  it("an in-ramp edit writes easing (and inMs when retimed), an out-ramp edit easing_out", () => {
    expect(patchOf("in", TWO)).toEqual({ easing: keysToString(TWO) });
    expect(patchOf("in", TWO, 420)).toEqual({ easing: keysToString(TWO), inMs: 420 });
    expect(patchOf("out", TWO, 700)).toEqual({ easing_out: keysToString(TWO), outMs: 700 });
  });

  it("the draft keeps the ramp it did not touch", () => {
    const next = withCurve(ZOOM, "out", THREE, 900);
    expect(next.rampIn).toEqual(ZOOM.rampIn);
    expect(next.rampOut).toEqual({ easing: keysToString(THREE), durMs: 900 });
    expect(withCurve(ZOOM, "in", THREE).rampIn.durMs).toBe(450);
  });
});

describe("toKeysInput", () => {
  it("converts a named curve so its dots show before the first drag", () => {
    const next = toKeysInput(ZOOM);
    expect(parseKeys(next.rampIn.easing)).not.toBeNull();
    expect(parseKeys(next.rampOut!.easing)).not.toBeNull();
    expect(evalKeys(parseKeys(next.rampIn.easing)!, 0.5)).toBeCloseTo(0.5, 3);
  });

  it("leaves a spring alone - it has no keys to draw", () => {
    const spr = toKeysInput({ ...ZOOM, rampIn: { easing: "spring(140,7,1)", durMs: 450 } });
    expect(spr.rampIn.easing).toBe("spring(140,7,1)");
  });

  it("leaves a curve that is already in keys form untouched", () => {
    const k = keysToString(THREE);
    expect(toKeysInput({ ...ZOOM, rampIn: { easing: k, durMs: 450 } }).rampIn.easing).toBe(k);
  });
});

describe("useGraphDrag, mounted", () => {
  it("commits a keyboard nudge on the focused key as one canonical patch", () => {
    const patches: GraphPatch[] = [];
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(MotionGraph, { input: ZOOM, onCommit: (p) => patches.push(p) })));

    const dots = host.querySelectorAll<SVGCircleElement>(".e-mg-dot");
    expect(dots).toHaveLength(4);
    expect(dots[0].getAttribute("aria-label")).toBe("In ramp key 1");
    act(() => {
      dots[1].dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    });
    act(() => {
      dots[1].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });

    expect(patches).toHaveLength(1);
    expect(patches[0].easing).toMatch(/^keys\(/);
    expect(parseKeys(patches[0].easing!)!.keys[1].v).toBeCloseTo(1.01, 3);
    expect(patches[0].easing_out).toBeUndefined();
    act(() => root.unmount());
    host.remove();
  });
});
