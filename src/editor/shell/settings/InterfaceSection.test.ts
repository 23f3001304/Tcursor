// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { DEFAULT_INTERFACE_RESET } from "./InterfaceSection";

describe("DEFAULT_INTERFACE_RESET (Task 35 - EditorSettingsDialog per-section reset)", () => {
  it("mirrors InterfaceSettings::default() exactly (settings/model.rs)", () => {
    expect(DEFAULT_INTERFACE_RESET).toEqual({
      theme: "light",
      accent: [239, 68, 68],
      animated_brand: true,
      ai_choreography: false,
    });
  });

  it("owns the accent too, since the swatches moved here from the Background panel (2026-09-15)", () => {
    expect(Object.keys(DEFAULT_INTERFACE_RESET).sort()).toEqual([
      "accent",
      "ai_choreography",
      "animated_brand",
      "theme",
    ]);
    expect(DEFAULT_INTERFACE_RESET.accent).toEqual([239, 68, 68]);
  });
});
