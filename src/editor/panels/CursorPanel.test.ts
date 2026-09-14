import { describe, it, expect } from "vitest";
import { DEFAULT_CURSOR_SETTINGS } from "./CursorPanel";

describe("DEFAULT_CURSOR_SETTINGS (Task 26 commit 1 - CursorPanel reset default bug)", () => {
  it("mirrors the Rust CursorSettings::default() exactly, including style: \"system\"", () => {
    // A prior bug wrote style: "enhanced" here, silently diverging from the backend default
    // (settings/model.rs: `CursorStyle::System`) every time a user pressed Reset.
    expect(DEFAULT_CURSOR_SETTINGS).toEqual({
      style: "system",
      size: 1.0,
      smoothness: 0.6,
      path_idealize: 0.0,
      motion_blur: 0.35,
      tilt: 0.35,
      click_bounce: true,
      bounce_intensity: 0.5,
      pack: "default",
      back: "none",
    });
  });
});
