// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import {
  hudLogicalSize,
  takeWidth,
  WIDTH,
  IDLE_HEIGHT,
  MENU_OVERFLOW,
  TAKE_WIDTH,
  TAKE_HEIGHT,
  FULL_PILL,
  SOURCES_HEIGHT,
  SOURCES_MENU_OVERFLOW,
  type PillParts,
} from "./useHudWindowSize";

const shut = (p: Partial<PillParts>): PillParts => ({ ...FULL_PILL, ...p });

describe("hudLogicalSize", () => {
  it("is the card's own size with no menu open, idle", () => {
    expect(hudLogicalSize(null, "idle")).toEqual({ width: WIDTH, height: IDLE_HEIGHT });
    expect(WIDTH).toBe(360);
  });

  it("grows by the menu overflow for any open dropdown, width unchanged", () => {
    for (const m of ["cam", "mic"])
      expect(hudLogicalSize(m, "idle")).toEqual({ width: WIDTH, height: IDLE_HEIGHT + MENU_OVERFLOW });
  });

  it("switches to the take pill's size while recording, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "recording")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
    expect(hudLogicalSize("cam", "recording")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
  });

  it("keeps the take pill's size while saving, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "saving")).toEqual(hudLogicalSize(null, "recording"));
    expect(hudLogicalSize("mic", "saving")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
  });

  it("the pill is a fraction of the card's height - the whole point of the resize", () => {
    expect(TAKE_HEIGHT).toBeLessThan(IDLE_HEIGHT / 4);
  });

  it("the everything-on pill is 473 wide, three 42px buttons included", () => {
    expect(TAKE_WIDTH).toBe(473);
  });

  it("the pill's window is sized for what it shows: no webcam circle or level slot for a source that is off", () => {
    const full = takeWidth(FULL_PILL);
    expect(full).toBe(TAKE_WIDTH);
    expect(takeWidth(shut({ cam: false }))).toBe(full - 44 - 10);
    expect(takeWidth(shut({ meter: false }))).toBe(full - (4 + 150) - 10);
    expect(takeWidth(shut({ warn: true }))).toBe(full + 20 + 10);
    expect(hudLogicalSize(null, "recording", shut({ cam: false, meter: false })).width).toBeLessThan(full);
  });

  it("the Sources sheet adds its own height and nothing else", () => {
    const open = shut({ sources: true });
    expect(hudLogicalSize(null, "recording", open)).toEqual({
      width: TAKE_WIDTH,
      height: TAKE_HEIGHT + SOURCES_HEIGHT,
    });
    expect(takeWidth(open)).toBe(TAKE_WIDTH);
    expect(SOURCES_HEIGHT).toBeGreaterThan(0);
  });

  it("a menu opened from the sheet grows the window; the same menu without the sheet does not", () => {
    expect(hudLogicalSize("src-mic", "recording", shut({ sources: true })).height).toBe(
      TAKE_HEIGHT + SOURCES_HEIGHT + SOURCES_MENU_OVERFLOW,
    );
    expect(hudLogicalSize("src-mic", "recording", FULL_PILL).height).toBe(TAKE_HEIGHT);
  });

  it("Saving is the bare pill again - Hud closes the sheet, and the size follows", () => {
    expect(hudLogicalSize(null, "saving", FULL_PILL).height).toBe(TAKE_HEIGHT);
  });
});
