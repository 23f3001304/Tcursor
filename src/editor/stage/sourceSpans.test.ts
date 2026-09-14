import { describe, it, expect } from "vitest";
import { FULL_SRC, activeSpanIdx, fitPanel, spanAt, toPanelFrac } from "./sourceSpans";
import { ease } from "../timeline/layoutTrack";
import type { SourceSpanDto } from "../../lib/ipc";

const FULL: SourceSpanDto = { start_ms: 0, src: [0, 0, 1, 1], transition_ms: 350, fit: [1, 1] };
// A 16:10 display fitted into a 16:9 canvas: 96px bars each side of 1920, and a panel 1536/1706.7
// wide where the canvas one was full width - the real numbers `frame_fit::letterbox` produces.
const WIDE16x10: SourceSpanDto = { start_ms: 3000, src: [0.05, 0, 0.9, 1], transition_ms: 350, fit: [0.9, 1] };

describe("spanAt", () => {
  it("is the whole canvas with no squeeze for a take that never switched display", () => {
    for (const t of [0, 500, 9999]) {
      expect(spanAt([FULL], t, ease)).toEqual({ src: FULL_SRC, fit: [1, 1] });
      expect(spanAt([], t, ease)).toEqual({ src: FULL_SRC, fit: [1, 1] }); // presets not loaded yet
    }
  });

  it("switches the crop rect the instant the switch lands, and only eases the panel", () => {
    const spans = [FULL, WIDE16x10];
    expect(spanAt(spans, 2999, ease).src).toEqual([0, 0, 1, 1]);
    // At the switch the content is ALREADY the new rect (Rust pre-blends the old picture into it)
    // while the panel still has the old aspect.
    const at = spanAt(spans, 3000, ease);
    expect(at.src).toEqual([0.05, 0, 0.9, 1]);
    expect(at.fit[0]).toBeCloseTo(1, 5);
    expect(spanAt(spans, 3175, ease).fit[0]).toBeCloseTo(0.95, 2); // smoothstep midpoint
    expect(spanAt(spans, 3350, ease).fit).toEqual([0.9, 1]);
    expect(spanAt(spans, 5000, ease).fit).toEqual([0.9, 1]);
  });

  it("narrows the panel monotonically across the transition", () => {
    const spans = [FULL, WIDE16x10];
    let last = Infinity;
    for (const t of [3000, 3050, 3100, 3175, 3250, 3349]) {
      const f = spanAt(spans, t, ease).fit[0];
      expect(f).toBeLessThanOrEqual(last + 1e-6);
      last = f;
    }
  });

  it("picks the last span that has started", () => {
    expect(activeSpanIdx([FULL, WIDE16x10], 2999)).toBe(0);
    expect(activeSpanIdx([FULL, WIDE16x10], 3000)).toBe(1);
    expect(activeSpanIdx([], 0)).toBe(-1);
  });
});

describe("fitPanel", () => {
  it("is the identity at [1, 1] - the no-switch pin for every existing recording", () => {
    const r: [number, number, number, number] = [0.1, 0.2, 0.7, 0.6];
    expect(fitPanel(r, [1, 1])).toEqual(r);
  });

  it("squeezes about the panel's own centre, so the panel stays put as it changes aspect", () => {
    const [x, y, w, h] = fitPanel([0.1, 0.2, 0.8, 0.6], [0.5, 1]);
    expect(w).toBeCloseTo(0.4, 6);
    expect(h).toBeCloseTo(0.6, 6);
    expect(x + w / 2).toBeCloseTo(0.5, 6); // same centre as the 0.1..0.9 rect it came from
    expect(y).toBeCloseTo(0.2, 6);
  });
});

describe("toPanelFrac", () => {
  it("is the identity for the whole canvas", () => {
    expect(toPanelFrac(0.25, 0.75, FULL_SRC)).toEqual([0.25, 0.75]);
  });

  it("maps the crop rect's edges onto the panel's, so a click lands on the cropped picture", () => {
    const src: [number, number, number, number] = [0.05, 0, 0.9, 1];
    expect(toPanelFrac(0.05, 0, src)[0]).toBeCloseTo(0, 6);
    expect(toPanelFrac(0.5, 0.5, src)[0]).toBeCloseTo(0.5, 6);
    expect(toPanelFrac(0.95, 1, src)[0]).toBeCloseTo(1, 6);
    // A point on the bar the crop removed reads outside the panel rather than being clamped onto it.
    expect(toPanelFrac(0.01, 0.5, src)[0]).toBeLessThan(0);
  });
});
