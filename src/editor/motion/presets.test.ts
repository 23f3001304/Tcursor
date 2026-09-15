import { describe, it, expect } from "vitest";
import { PRESETS, presetOf, presetPatch, type MotionPreset } from "./presets";

describe("PRESETS (M3 - the five feels, as data)", () => {
  it("is the spec's five, in the spec's order", () => {
    expect(PRESETS.map((p) => p.id)).toEqual(["snappy", "soft", "cinematic", "mechanical", "bouncy"]);
  });

  it("gives Soft the bare word smooth for both ramps, not an equivalent keys curve", () => {
    const soft = PRESETS.find((p) => p.id === "soft")!;
    expect([soft.easing, soft.easing_out]).toEqual(["smooth", "smooth"]);
  });

  it("splits only Cinematic's two ramps - every other preset is symmetric", () => {
    const split = PRESETS.filter((p) => p.easing !== p.easing_out).map((p) => p.id);
    expect(split).toEqual(["cinematic"]);
  });

  it("carries the spec's exact strings", () => {
    const by = (id: MotionPreset["id"]) => PRESETS.find((p) => p.id === id)!;
    expect(by("snappy").easing).toBe("keys(0 0 0 0 0.1 0.7 b,1 1 -0.4 0 0 0 b)");
    expect(by("cinematic").easing).toBe("keys(0 0 0 0 0.45 0 b,1 1 -0.25 0 0 0 b)");
    expect(by("cinematic").easing_out).toBe("keys(0 0 0 0 0.15 -0.06 b,1 1 -0.4 0 0 0 b)");
    expect(by("mechanical").easing).toBe("keys(0 0 0 0 0 0 l,0.85 1 0 0 0 0 h,1 1 0 0 0 0 l)");
    expect(by("bouncy").easing).toBe("spring(140,7,1)");
  });

  it("gives every preset a name and a feel line for the picker", () => {
    for (const p of PRESETS) {
      expect(p.name.length).toBeGreaterThan(0);
      expect(p.feel.length).toBeGreaterThan(0);
    }
  });
});

describe("presetOf", () => {
  it("names every preset from its own pair", () => {
    for (const p of PRESETS) expect(presetOf(p.easing, p.easing_out)).toBe(p.id);
  });

  it('reads a pre-M3 region - easing "smooth", no easing_out at all - as Soft, never Custom', () => {
    expect(presetOf("smooth", undefined)).toBe("soft");
    expect(presetOf("smooth", null)).toBe("soft");
    expect(presetOf("smooth", "")).toBe("soft");
    expect(presetOf("smooth", "smooth")).toBe("soft");
  });

  it("names Bouncy from the canonical spring Rust writes back, as well as the short literal", () => {
    expect(presetOf("spring(140,7,1)", "spring(140,7,1)")).toBe("bouncy");
    expect(presetOf("spring(140.000,7.000,1.000)", "spring(140.000,7.000,1.000)")).toBe("bouncy");
    expect(presetOf("spring(140,7)", "spring(140,7)")).toBe("bouncy");
  });

  it("is custom for a curve no preset has, and for a preset's halves mismatched", () => {
    expect(presetOf("linear", "linear")).toBe("custom");
    expect(presetOf("spring(100,10,1)", "spring(100,10,1)")).toBe("custom");
    const snappy = PRESETS.find((p) => p.id === "snappy")!;
    const cine = PRESETS.find((p) => p.id === "cinematic")!;
    expect(presetOf(snappy.easing, cine.easing_out)).toBe("custom");
    expect(presetOf(cine.easing, undefined)).toBe("custom");
  });
});

describe("presetPatch", () => {
  it("returns exactly the preset's two strings", () => {
    for (const p of PRESETS)
      expect(presetPatch(p.id)).toEqual({ easing: p.easing, easing_out: p.easing_out });
  });

  it("round-trips through presetOf", () => {
    for (const p of PRESETS) {
      const patch = presetPatch(p.id);
      expect(presetOf(patch.easing, patch.easing_out)).toBe(p.id);
    }
  });

  it("falls back to Soft for an id that is not a preset, never undefined", () => {
    expect(presetPatch("custom")).toEqual({ easing: "smooth", easing_out: "smooth" });
    expect(presetPatch("")).toEqual({ easing: "smooth", easing_out: "smooth" });
  });
});

describe("presetOf against the canonical keys(...) form", () => {
  it("names Snappy from the 3-decimal string Rust's valid_easing writes back, not only the short literal", () => {
    const canon = "keys(0.000 0.000 0.000 0.000 0.100 0.700 b,1.000 1.000 -0.400 0.000 0.000 0.000 b)";
    expect(presetOf(canon, canon)).toBe("snappy");
    expect(presetOf(canon, undefined)).toBe("snappy");
  });
});
