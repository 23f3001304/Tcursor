import { describe, it, expect } from "vitest";
import { rowNextIndex } from "./TileRow";

describe("rowNextIndex", () => {
  it("steps forward and back along the row", () => {
    expect(rowNextIndex("ArrowRight", 0, 4)).toBe(1);
    expect(rowNextIndex("ArrowLeft", 2, 4)).toBe(1);
  });

  it("clamps at both ends rather than wrapping", () => {
    // The opposite of `segmentedNextIndex`, on purpose: a row can hold a dozen tiles, so running
    // off the end should stop where the eye is, not teleport to the far side of the strip.
    expect(rowNextIndex("ArrowRight", 3, 4)).toBe(3);
    expect(rowNextIndex("ArrowLeft", 0, 4)).toBe(0);
  });

  it("treats the vertical arrows the same as the horizontal ones", () => {
    expect(rowNextIndex("ArrowDown", 0, 4)).toBe(1);
    expect(rowNextIndex("ArrowUp", 2, 4)).toBe(1);
  });

  it("jumps to the ends on Home/End and ignores everything else", () => {
    expect(rowNextIndex("Home", 3, 4)).toBe(0);
    expect(rowNextIndex("End", 0, 4)).toBe(3);
    expect(rowNextIndex("Enter", 0, 4)).toBeNull();
    expect(rowNextIndex(" ", 0, 4)).toBeNull();
  });

  it("starts from the first tile when this row holds no selection, and refuses an empty row", () => {
    expect(rowNextIndex("ArrowRight", -1, 4)).toBe(1);
    expect(rowNextIndex("ArrowLeft", -1, 4)).toBe(0);
    expect(rowNextIndex("ArrowRight", -1, 0)).toBeNull();
  });
});
