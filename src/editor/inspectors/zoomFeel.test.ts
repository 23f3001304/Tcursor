import { describe, it, expect } from "vitest";
import { FEEL_PRESETS, activeFeel, feelPatch } from "./zoomFeel";

// The Feel row's whole contract: three named presets, a match that is exact on all three fields,
// and "no preset" (not a fourth "Custom" preset) for anything else - which is what lets the
// segmented row render with nothing selected instead of lighting one falsely.
describe("FEEL_PRESETS", () => {
  it("is the three named feels, in order, quiet to loud", () => {
    expect(FEEL_PRESETS.map((p) => p.name)).toEqual(["Subtle", "Balanced", "Punchy"]);
  });

  it("gets faster and springier as it gets louder", () => {
    const ins = FEEL_PRESETS.map((p) => p.zoom_in_ms);
    const outs = FEEL_PRESETS.map((p) => p.zoom_out_ms);
    expect(ins).toEqual([...ins].sort((a, b) => b - a));
    expect(outs).toEqual([...outs].sort((a, b) => b - a));
    expect(FEEL_PRESETS[2].easing).toBe("spring");
  });

  it("gives every preset its own easing, so the row is about feel and not only duration", () => {
    expect(new Set(FEEL_PRESETS.map((p) => p.easing)).size).toBe(3);
  });

  it("touches the two durations and the easing, and never the scale", () => {
    for (const p of FEEL_PRESETS) {
      expect(Object.keys(feelPatch(p.name) ?? {}).sort()).toEqual(["easing", "zoom_in_ms", "zoom_out_ms"]);
    }
  });
});

describe("activeFeel", () => {
  it("names the preset a zoom matches on all three fields", () => {
    for (const p of FEEL_PRESETS) {
      expect(activeFeel({ zoom_in_ms: p.zoom_in_ms, zoom_out_ms: p.zoom_out_ms, easing: p.easing })).toBe(p.name);
    }
  });

  it("is null (no plane lit) when any one field is off the preset", () => {
    const b = FEEL_PRESETS[1];
    expect(activeFeel({ ...b, zoom_in_ms: b.zoom_in_ms + 10 })).toBeNull();
    expect(activeFeel({ ...b, zoom_out_ms: b.zoom_out_ms - 1 })).toBeNull();
    expect(activeFeel({ ...b, easing: "linear" })).toBeNull();
  });

  it("ignores the scale, which is Framing's hero and not part of a feel", () => {
    const b = FEEL_PRESETS[1];
    expect(activeFeel({ ...b, scale: 3.9 } as never)).toBe("Balanced");
  });
});

describe("feelPatch", () => {
  it("is the exact update_zoom payload the row applies", () => {
    expect(feelPatch("Punchy")).toEqual({ zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" });
  });

  it("is null for a name that is not a preset, so an unknown key applies nothing", () => {
    expect(feelPatch("Custom")).toBeNull();
    expect(feelPatch("")).toBeNull();
  });
});
