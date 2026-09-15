import { describe, expect, it } from "vitest";
import { DEFAULT_CAPTION_STYLE } from "./CaptionStyleControls";
import { LINE_PCT_MAX, LINE_PCT_MIN, SIZE_RUNG_PCT, captionLinePct, captionRung } from "./captionLook";
import type { CaptionStyle } from "../../../hud/settings/settings";

const style = (o: Partial<CaptionStyle>): CaptionStyle => ({ ...DEFAULT_CAPTION_STYLE, ...o });

describe("the caption size rungs and the line-height slider", () => {
  it("reads a project that never touched the slider off its size rung", () => {
    expect(captionLinePct(style({ size: "m", font_pct: 0 }))).toBe(3.8);
    expect(captionLinePct(style({ size: "s", font_pct: 0 }))).toBe(3.0);
    expect(captionLinePct(style({ size: "l", font_pct: 0 }))).toBe(4.8);
  });

  it("lights the rung the fine size sits exactly on, and no rung between two", () => {
    expect(captionRung(style({ size: "m", font_pct: 0 }))).toBe("m");
    expect(captionRung(style({ size: "m", font_pct: 4.8 }))).toBe("l");
    expect(captionRung(style({ size: "m", font_pct: 4.2 }))).toBe("");
  });

  it("puts every rung inside the slider's own range, so picking one never pins the thumb", () => {
    for (const pct of Object.values(SIZE_RUNG_PCT)) {
      expect(pct).toBeGreaterThanOrEqual(LINE_PCT_MIN);
      expect(pct).toBeLessThanOrEqual(LINE_PCT_MAX);
    }
  });

  it("keeps the slider showing what the rung wrote, so the two never disagree", () => {
    const picked = style({ size: "s", font_pct: SIZE_RUNG_PCT.s });
    expect(captionLinePct(picked)).toBe(SIZE_RUNG_PCT.s);
    expect(captionRung(picked)).toBe("s");
  });
});
