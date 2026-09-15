// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { stylesMirrored, overlayNeedsClicks, drawRipplePreview, drawMirroredRipples } from "./ripplePreview";
import type { ClickSample } from "../../../shared/ipc";

const untouchableCtx = new Proxy(
  {},
  {
    get(_t, prop) {
      throw new Error(`drawRipplePreview touched ctx.${String(prop)} on a no-op path`);
    },
  },
) as unknown as CanvasRenderingContext2D;

const clk = (t: number, x: number, y: number): ClickSample => ({ t, x, y });

describe("stylesMirrored and overlayNeedsClicks (fxOverlay.ts's fallback gate)", () => {
  it("mirrors ripple (the default), shockwave and pulse, which then need no overlay hits", () => {
    for (const s of ["ripple", "shockwave", "pulse"]) {
      expect(stylesMirrored.has(s)).toBe(true);
      expect(overlayNeedsClicks(s)).toBe(false);
    }
  });

  it("leaves the other three to the backend overlay, which must still carry their hits", () => {
    for (const s of ["glow", "particles", "neon"]) {
      expect(stylesMirrored.has(s)).toBe(false);
      expect(overlayNeedsClicks(s)).toBe(true);
    }
  });

  it('treats "none" as neither: nothing to draw either way', () => {
    expect(stylesMirrored.has("none")).toBe(false);
    expect(overlayNeedsClicks("none")).toBe(false);
  });
});

describe("drawRipplePreview no-op paths (never touch ctx)", () => {
  it("does nothing for a style outside stylesMirrored", () => {
    expect(() =>
      drawRipplePreview(untouchableCtx, [{ x: 1, y: 1, progress: 0.5 }], "glow", [255, 255, 255], 1, 1000),
    ).not.toThrow();
  });

  it("does nothing for an empty hit list or a non-positive output height", () => {
    expect(() => drawRipplePreview(untouchableCtx, [], "ripple", [255, 255, 255], 1, 1000)).not.toThrow();
    expect(() =>
      drawRipplePreview(untouchableCtx, [{ x: 1, y: 1, progress: 0.5 }], "ripple", [255, 255, 255], 1, 0),
    ).not.toThrow();
  });
});

describe("drawMirroredRipples no-op paths (never touch ctx)", () => {
  const identity = (x: number, y: number): [number, number] => [x, y];
  const activeClick = [clk(0, 0.5, 0.5)];

  it("does nothing when clickfx is disabled, even for an active click on a mirrored style", () => {
    expect(() =>
      drawMirroredRipples(
        untouchableCtx,
        activeClick,
        0,
        false,
        "ripple",
        [255, 255, 255],
        1,
        identity,
        100,
        100,
        200,
        200,
      ),
    ).not.toThrow();
  });

  it("does nothing for a style outside stylesMirrored, even when enabled", () => {
    expect(() =>
      drawMirroredRipples(
        untouchableCtx,
        activeClick,
        0,
        true,
        "glow",
        [255, 255, 255],
        1,
        identity,
        100,
        100,
        200,
        200,
      ),
    ).not.toThrow();
  });

  it("does nothing for an empty click list, even when enabled", () => {
    expect(() =>
      drawMirroredRipples(
        untouchableCtx,
        [],
        0,
        true,
        "ripple",
        [255, 255, 255],
        1,
        identity,
        100,
        100,
        200,
        200,
      ),
    ).not.toThrow();
  });
});
