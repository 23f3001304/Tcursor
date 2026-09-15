// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { DEFAULT_MOTION_RESET, smoothingLagMs } from "./MotionSection";
import { DEFAULT_ZOOM_SETTINGS } from "./ZoomDefaultsSection";
import { presetOf } from "../../motion/presets";

describe("DEFAULT_MOTION_RESET (M3 - Settings > Motion's per-section reset)", () => {
  it("mirrors Rust MotionSettings::default() exactly (settings/motion.rs)", () => {
    expect(DEFAULT_MOTION_RESET).toEqual({ preset: "soft", easing: "smooth", easing_out: "smooth" });
  });

  it("carries every field of the struct - Reset replaces the whole object", () => {
    expect(Object.keys(DEFAULT_MOTION_RESET).sort()).toEqual(["easing", "easing_out", "preset"]);
  });

  it("is a pair the picker reads back as Soft, not as Custom", () => {
    expect(presetOf(DEFAULT_MOTION_RESET.easing, DEFAULT_MOTION_RESET.easing_out)).toBe("soft");
  });
});

describe("smoothingLagMs (the knob's lag readout, moved here from Zoom defaults)", () => {
  it("is the slope the jank probe measured - 120 ms costs about 33 ms, 250 ms about 83 ms", () => {
    expect(smoothingLagMs(120)).toBe(36);
    expect(smoothingLagMs(250)).toBe(75);
    expect(smoothingLagMs(0)).toBe(0);
  });

  it("rounds to whole milliseconds", () => {
    expect(smoothingLagMs(10)).toBe(3);
    expect(smoothingLagMs(400)).toBe(120);
  });
});

describe("the smoothing knob's field stays on ZoomSettings", () => {
  it("only the CONTROL moved - DEFAULT_ZOOM_SETTINGS must still carry camera_smoothing_ms", () => {
    expect(DEFAULT_ZOOM_SETTINGS.camera_smoothing_ms).toBe(0);
  });
});
