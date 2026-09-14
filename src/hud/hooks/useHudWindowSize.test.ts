import { describe, it, expect } from "vitest";
import { hudLogicalSize, takeWidth, WIDTH, IDLE_HEIGHT, MENU_OVERFLOW, TAKE_WIDTH, TAKE_HEIGHT, FULL_PILL,
  SOURCES_HEIGHT, SOURCES_MENU_OVERFLOW, type PillParts } from "./useHudWindowSize";

/** The pill as it is with the Sources sheet shut - what every pre-sheet assertion below means. */
const shut = (p: Partial<PillParts>): PillParts => ({ ...FULL_PILL, ...p });

// Regression coverage for the HUD right-edge clipping bug: the window's LogicalSize and the
// hud.css `.hud` card must agree on the *same* numbers, or a mismatched/stale DPI scale factor
// renders content wider than the physical window. This locks the one function both the
// menu-driven resize and the onScaleChanged re-normalization call, so they can never diverge.
describe("hudLogicalSize", () => {
  it("is the card's own size with no menu open, idle", () => {
    expect(hudLogicalSize(null, "idle")).toEqual({ width: WIDTH, height: IDLE_HEIGHT });
    expect(WIDTH).toBe(360);
  });

  it("grows by the menu overflow for any open dropdown, width unchanged", () => {
    for (const m of ["cam", "mic"]) expect(hudLogicalSize(m, "idle")).toEqual({ width: WIDTH, height: IDLE_HEIGHT + MENU_OVERFLOW });
  });

  // D5 (user-reported 2026-09-02): the recording bar used to keep the idle window around its
  // much narrower content. A take gets the pill's own content-fit size, regardless of a stale
  // `menu` value left over from before Record was pressed (no dropdown can be open in a take -
  // Hud.tsx only renders them while idle).
  it("switches to the take pill's size while recording, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "recording")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
    expect(hudLogicalSize("cam", "recording")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
  });

  // Item 3 (user-reported 2026-09-02): "Saving… X%" used to fall back to the idle window. The
  // saving pill is the SAME size as the recording one, so Stop never resizes the window on its
  // way into the editor.
  it("keeps the take pill's size while saving, ignoring a stale menu value", () => {
    expect(hudLogicalSize(null, "saving")).toEqual(hudLogicalSize(null, "recording"));
    expect(hudLogicalSize("mic", "saving")).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT });
  });

  it("the pill is a fraction of the card's height - the whole point of the resize", () => {
    expect(TAKE_HEIGHT).toBeLessThan(IDLE_HEIGHT / 4);
  });

  // Pinned so the number quoted in TakeBar.md and the derivation above can never quietly drift
  // apart (they already had, before the Sources button was added).
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

  // Mid-take source switching (2026-09-14): Sources hangs a sheet UNDER the pill, so the window
  // gets taller and the pill itself must not move or change shape.
  it("the Sources sheet adds its own height and nothing else", () => {
    const open = shut({ sources: true });
    expect(hudLogicalSize(null, "recording", open)).toEqual({ width: TAKE_WIDTH, height: TAKE_HEIGHT + SOURCES_HEIGHT });
    expect(takeWidth(open)).toBe(TAKE_WIDTH);
    expect(SOURCES_HEIGHT).toBeGreaterThan(0);
  });

  it("a menu opened from the sheet grows the window; the same menu without the sheet does not", () => {
    expect(hudLogicalSize("src-mic", "recording", shut({ sources: true })).height)
      .toBe(TAKE_HEIGHT + SOURCES_HEIGHT + SOURCES_MENU_OVERFLOW);
    expect(hudLogicalSize("src-mic", "recording", FULL_PILL).height).toBe(TAKE_HEIGHT);
  });

  it("Saving is the bare pill again - Hud closes the sheet, and the size follows", () => {
    expect(hudLogicalSize(null, "saving", FULL_PILL).height).toBe(TAKE_HEIGHT);
  });
});
