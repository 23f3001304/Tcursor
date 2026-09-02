import { describe, it, expect } from "vitest";
import { hudLogicalSize, WIDTH, RECORDING_WIDTH, SAVING_WIDTH } from "./useHudWindowSize";

// Regression coverage for the HUD right-edge clipping bug: the window's LogicalSize and the
// hud.css `.hud` bar must agree on the *same* numbers, or a mismatched/stale DPI scale factor
// renders content wider than the physical window. This locks the one function both the
// menu-driven resize and the onScaleChanged re-normalization call, so they can never diverge.
describe("hudLogicalSize", () => {
  it("is 980x132 with no menu open, idle", () => {
    expect(hudLogicalSize(null, "idle")).toEqual({ width: WIDTH, height: 132 });
  });

  it("grows to 430 tall for any open dropdown, width unchanged", () => {
    expect(hudLogicalSize("cam", "idle")).toEqual({ width: WIDTH, height: 430 });
    expect(hudLogicalSize("screen", "idle")).toEqual({ width: WIDTH, height: 430 });
    expect(hudLogicalSize("mic", "idle")).toEqual({ width: WIDTH, height: 430 });
  });

  it("width is always WIDTH regardless of menu state when idle - the bar never changes width, only height", () => {
    expect(hudLogicalSize(null, "idle").width).toBe(WIDTH);
    expect(hudLogicalSize("cam", "idle").width).toBe(WIDTH);
  });

  // D5 (user-reported 2026-09-02): the recording bar used to keep the idle 980px window around
  // its much narrower content. `mode: "recording"` switches to the content-fit RECORDING_WIDTH,
  // always at height 132 (no dropdown menu can be open while recording - Hud.tsx only renders
  // them while idle), regardless of a stale `menu` value left over from before Record was pressed.
  it("switches to the content-fit RECORDING_WIDTH while recording, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "recording")).toEqual({ width: RECORDING_WIDTH, height: 132 });
    expect(hudLogicalSize("cam", "recording")).toEqual({ width: RECORDING_WIDTH, height: 132 });
  });

  // Item 3 (user-reported 2026-09-02): "Saving… X%" used to fall back to the idle 980px window
  // (mode was never threaded past "recording" vs not) around ~300px of actual content, since
  // `recording` is already false by the time `saving` goes true (useRecordingFlow.finish sets
  // both in the same render). `mode: "saving"` gets its own content-fit width, same shape as
  // recording's, also ignoring a stale `menu` value (no dropdown can be open while saving either).
  it("switches to the content-fit SAVING_WIDTH while saving, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "saving")).toEqual({ width: SAVING_WIDTH, height: 132 });
    expect(hudLogicalSize("mic", "saving")).toEqual({ width: SAVING_WIDTH, height: 132 });
  });

  it("RECORDING_WIDTH and SAVING_WIDTH are both meaningfully narrower than the idle WIDTH - the whole point of the resize", () => {
    expect(RECORDING_WIDTH).toBeLessThan(WIDTH);
    expect(SAVING_WIDTH).toBeLessThan(WIDTH);
  });

  it("SAVING_WIDTH is narrower than RECORDING_WIDTH - the saving row has strictly fewer/shorter children (no camera tile, timer, paused chip, or Pause button)", () => {
    expect(SAVING_WIDTH).toBeLessThan(RECORDING_WIDTH);
  });
});
