import { describe, it, expect } from "vitest";
import { TAU, crestBefore, sinePath, sineY, type SineSpec } from "./sine";

const spec = (o: Partial<SineSpec> = {}): SineSpec => ({
  w: 100,
  mid: 20,
  amp: 10,
  lambda: 40,
  phase: 0,
  ...o,
});

describe("sineY", () => {
  it("is the midline at zero amplitude, everywhere", () => {
    const s = spec({ amp: 0 });
    for (let x = 0; x <= s.w; x += 7) expect(sineY(s, x)).toBe(s.mid);
  });

  it("puts the crest a full amplitude ABOVE the midline a quarter period in", () => {
    const s = spec();
    expect(sineY(s, 0)).toBeCloseTo(s.mid, 10);
    expect(sineY(s, s.lambda / 4)).toBeCloseTo(s.mid - s.amp, 10);
    expect(sineY(s, (3 * s.lambda) / 4)).toBeCloseTo(s.mid + s.amp, 10);
  });

  it("is a true sine: exactly periodic in lambda", () => {
    const s = spec({ phase: 0.7 });
    for (let x = 0; x < s.lambda; x += 3) expect(sineY(s, x + s.lambda)).toBeCloseTo(sineY(s, x), 10);
  });

  it("scales the amplitude by the envelope, never the midline", () => {
    const s = spec({ envelope: (t) => t });
    expect(sineY(s, 0)).toBeCloseTo(s.mid, 10);
    expect(sineY({ ...s, envelope: () => 0.5 }, s.lambda / 4)).toBeCloseTo(s.mid - s.amp / 2, 10);
  });
});

describe("sinePath", () => {
  it("samples from 0 to exactly w, whatever the step divides into", () => {
    const d = sinePath(spec({ w: 101, step: 4 }));
    expect(d.startsWith("M 0.00 ")).toBe(true);
    expect(d.split(" L ").length).toBe(Math.ceil(101 / 4) + 1);
    expect(d.endsWith(sineY(spec({ w: 101 }), 101).toFixed(2))).toBe(true);
  });

  it("draws a flat line at zero amplitude", () => {
    const ys = sinePath(spec({ amp: 0, w: 20, step: 5 }))
      .split(/[ML] /)
      .slice(1)
      .map((p) => p.trim().split(" ")[1]);
    expect(new Set(ys)).toEqual(new Set(["20.00"]));
  });
});

describe("crestBefore", () => {
  it("returns a real crest of the same wave, at or before the limit", () => {
    const s = spec({ phase: 1.3, lambda: 37 });
    const x = crestBefore(s, 90);
    expect(x).toBeLessThanOrEqual(90);
    expect(x).toBeGreaterThan(90 - s.lambda);
    expect(sineY(s, x)).toBeCloseTo(s.mid - s.amp, 8);
  });

  it("steps back by exactly one period as the limit crosses a crest", () => {
    const s = spec({ phase: 0, lambda: 40 });
    expect(crestBefore(s, 50.1)).toBeCloseTo(50, 8);
    expect(crestBefore(s, 49.9)).toBeCloseTo(10, 8);
  });
});

describe("TAU", () => {
  it("is a full turn", () => expect(TAU).toBeCloseTo(2 * Math.PI, 12));
});
