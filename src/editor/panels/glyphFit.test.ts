import { describe, it, expect } from "vitest";
import { alphaBounds, fitBox } from "./glyphFit";

/** An `w x h` RGBA buffer with one opaque rectangle painted into it. */
function buffer(w: number, h: number, rect: { x: number; y: number; w: number; h: number }): Uint8ClampedArray {
  const data = new Uint8ClampedArray(w * h * 4);
  for (let y = rect.y; y < rect.y + rect.h; y++) {
    for (let x = rect.x; x < rect.x + rect.w; x++) data[(y * w + x) * 4 + 3] = 255;
  }
  return data;
}

describe("alphaBounds", () => {
  it("finds the drawn content inside a transparently padded sprite", () => {
    expect(alphaBounds(buffer(16, 16, { x: 3, y: 5, w: 4, h: 6 }), 16, 16))
      .toEqual({ x: 3, y: 5, w: 4, h: 6 });
  });

  it("returns null for a fully transparent image, so the caller keeps its fallback", () => {
    expect(alphaBounds(new Uint8ClampedArray(8 * 8 * 4), 8, 8)).toBeNull();
  });

  it("ignores all-but-invisible pixels, which anti-aliased PNG edges are full of", () => {
    const data = buffer(8, 8, { x: 2, y: 2, w: 2, h: 2 });
    data[(0 * 8 + 7) * 4 + 3] = 3; // a stray alpha-3 pixel in the corner
    expect(alphaBounds(data, 8, 8)).toEqual({ x: 2, y: 2, w: 2, h: 2 });
  });
});

describe("fitBox", () => {
  // The point of the whole exercise: two packs whose drawings are different sizes inside the same
  // 128x128 canvas must come out the same visual size on the tile.
  it("scales by the content box, so differently padded packs match", () => {
    const small = fitBox({ x: 44, y: 40, w: 40, h: 48 }, 128, 128, 20, 24, 34, 34);
    const large = fitBox({ x: 14, y: 10, w: 100, h: 120 }, 128, 128, 20, 24, 34, 34);
    expect(small.width / (128 / 40)).toBeCloseTo(20, 10);   // both content boxes end up 20 wide
    expect(large.width / (128 / 100)).toBeCloseTo(20, 10);
  });

  it("fits the limiting axis and never distorts", () => {
    const fit = fitBox({ x: 0, y: 0, w: 50, h: 100 }, 100, 100, 20, 24, 34, 34);
    expect(fit.height / fit.width).toBeCloseTo(1, 10); // the image itself stays square
    expect(fit.width).toBeCloseTo(100 * (24 / 100), 10); // height was the binding constraint
  });

  it("centres the content box in the plate, not the image", () => {
    const fit = fitBox({ x: 0, y: 0, w: 20, h: 20 }, 40, 40, 20, 20, 34, 34);
    // content centre (10, 10) at scale 1 lands on the plate centre (17, 17)
    expect(fit.left).toBeCloseTo(7, 10);
    expect(fit.top).toBeCloseTo(7, 10);
  });
});
