// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { DEFAULT_SCREEN_SIZE } from "./ScreenSection";

describe("DEFAULT_SCREEN_SIZE (Task 35 - EditorSettingsDialog per-section reset)", () => {
  it("mirrors ModeAppearance::default().screen_size exactly (settings/appearance.rs)", () => {
    expect(DEFAULT_SCREEN_SIZE).toBe(1.0);
  });
});
