import { describe, it, expect } from "vitest";
import { paramsOf, seedOf, vignetteK } from "./gradeParams";

describe("paramsOf and the seeds", () => {
  it("returns null for the identity and something for a bent knob", () => {
    expect(paramsOf({ preset: "none", exposure: 0, contrast: 1, vignette: 0 })).toBeNull();
    expect(paramsOf({ preset: "none", exposure: 0.05, contrast: 1, vignette: 0 })).not.toBeNull();
  });

  it("seeds the three stored numbers from the preset row", () => {
    expect(seedOf("none")).toEqual([0, 1, 0]);
    expect(seedOf("cinematic")).toEqual([0, 1.12, 0.28]);
    expect(seedOf("midnight")).toEqual([-0.12, 1.18, 0.42]);
  });

  it("lets the stored numbers win while the other eight stay the preset's", () => {
    const p = paramsOf({ preset: "noir", exposure: -1, contrast: 0.5, vignette: 0 })!;
    expect([p.exposure, p.contrast, p.vignette]).toEqual([-1, 0.5, 0]);
    expect(p.saturation).toBe(0);
  });

  it("puts the vignette at zero in the centre and one at a corner", () => {
    expect(vignetteK(0, 0)).toBe(0);
    expect(vignetteK(0.5, 0.5)).toBe(1);
  });
});
