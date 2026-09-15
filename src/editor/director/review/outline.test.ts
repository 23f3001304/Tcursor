import { describe, it, expect } from "vitest";
import { outlineRect } from "./outline";
import type { PreviewLayout } from "../../../shared/ipc";

const FULL = { screen: [0, 0, 1, 1] } as unknown as PreviewLayout;

describe("stage outline", () => {
  it("is the identity when the screen fills the frame and nothing is zoomed", () => {
    expect(outlineRect([0.25, 0.2, 0.5, 0.4], 1280, 720, FULL, { scale: 1, cx: 0.5, cy: 0.5 })).toEqual({
      left: 0.25,
      top: 0.2,
      width: 0.5,
      height: 0.4,
    });
  });

  it("follows the crop when the camera is zoomed in", () => {
    const box = outlineRect([0.4, 0.4, 0.2, 0.2], 1280, 720, FULL, { scale: 2, cx: 0.5, cy: 0.5 })!;
    expect(box.left).toBeCloseTo(0.3, 3);
    expect(box.width).toBeCloseTo(0.4, 3);
  });

  it("returns null for a rect the current crop has pushed off frame", () => {
    expect(outlineRect([0, 0, 0.05, 0.05], 1280, 720, FULL, { scale: 4, cx: 0.9, cy: 0.9 })).toBeNull();
  });
});
