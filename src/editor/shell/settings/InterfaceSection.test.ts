import { describe, it, expect } from "vitest";
import { DEFAULT_INTERFACE_RESET } from "./InterfaceSection";

describe("DEFAULT_INTERFACE_RESET (Task 35 - EditorSettingsDialog per-section reset)", () => {
  it("mirrors InterfaceSettings::default()'s theme/animated_brand exactly (settings/model.rs)", () => {
    expect(DEFAULT_INTERFACE_RESET).toEqual({ theme: "light", animated_brand: true });
  });

  it("deliberately excludes accent - that stays BackgroundPanel's own reset target", () => {
    expect(Object.keys(DEFAULT_INTERFACE_RESET).sort()).toEqual(["animated_brand", "theme"]);
    expect(DEFAULT_INTERFACE_RESET).not.toHaveProperty("accent");
  });
});
