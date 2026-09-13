import { describe, it, expect } from "vitest";
import { fromHex, toHex } from "./ColorInput";

describe("toHex / fromHex (BackgroundPanel's custom color inputs)", () => {
  it("round-trips every channel, zero-padding single-digit components", () => {
    expect(toHex([0, 0, 0])).toBe("#000000");
    expect(toHex([255, 255, 255])).toBe("#ffffff");
    expect(toHex([76, 123, 255])).toBe("#4c7bff"); // the brand blue
    expect(fromHex("#4c7bff")).toEqual([76, 123, 255]);
    expect(fromHex(toHex([1, 2, 3]))).toEqual([1, 2, 3]);
  });

  it("clamps and rounds out-of-range channels instead of emitting an invalid hex", () => {
    // `<input type="color">` silently ignores a malformed value, which would strand the swatch on
    // a stale colour - so the conversion has to produce six valid digits for any input.
    expect(toHex([-5, 300, 12.6])).toBe("#00ff0d");
  });

  it("treats anything that is not a #rrggbb string as black rather than throwing", () => {
    expect(fromHex("")).toEqual([0, 0, 0]);
    expect(fromHex("red")).toEqual([0, 0, 0]);
    expect(fromHex("#abc")).toEqual([0, 0, 0]);
    expect(fromHex("  #FFAA00 ")).toEqual([255, 170, 0]); // trimmed and case-insensitive
  });
});
