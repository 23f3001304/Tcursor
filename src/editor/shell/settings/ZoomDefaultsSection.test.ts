import { describe, it, expect } from "vitest";
import { DEFAULT_ZOOM_SETTINGS, clicksToOption, clicksFromOption } from "./ZoomDefaultsSection";

describe("DEFAULT_ZOOM_SETTINGS (Task 35 - EditorSettingsDialog per-section reset)", () => {
  it("mirrors the Rust ZoomSettings::default() exactly (settings/model.rs)", () => {
    expect(DEFAULT_ZOOM_SETTINGS).toEqual({
      enabled: true,
      target_scale: 2.2,
      hold_ms: 2200,
      smoothness: 0.10,
      clicks: 1,
      camera_shrink: true,
      camera_shrink_min: 0.62,
      smart_hold: true,
      smart_follow: false,
      cam_zoom_default: null,
      camera_smoothing_ms: 0,
    });
  });
});

describe("clicksToOption / clicksFromOption (the clicks:u32 <-> 3-way Picker mapping, Task 35 fix)", () => {
  it("reads clicks:1 (the Rust default) as \"1\"", () => {
    expect(clicksToOption(1)).toBe("1");
  });
  it("reads clicks:2 as \"2\"", () => {
    expect(clicksToOption(2)).toBe("2");
  });
  it("reads clicks:3 (a real HUD-seeded value) as \"3\" - not collapsed to 1 or 2", () => {
    expect(clicksToOption(3)).toBe("3");
  });
  it("writes each option straight through to its number - no lossy transform", () => {
    expect(clicksFromOption("1")).toBe(1);
    expect(clicksFromOption("2")).toBe(2);
    expect(clicksFromOption("3")).toBe(3);
  });
  it("round-trips clicks:3 all the way through (selecting \"3\" writes clicks:3) - the bug this fix closes: a prior Switch-based control silently collapsed 3 to 1 or 2", () => {
    expect(clicksFromOption(clicksToOption(3))).toBe(3);
  });
});
