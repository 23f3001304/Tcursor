// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { snapToStep, sliderKeyValue } from "./Slider";

describe("snapToStep", () => {
  it("snaps to the nearest step and clamps to [min, max]", () => {
    expect(snapToStep(0.27, 0, 1, 0.1)).toBeCloseTo(0.3, 10);
    expect(snapToStep(-5, 0, 1, 0.1)).toBe(0);
    expect(snapToStep(5, 0, 1, 0.1)).toBe(1);
  });

  it("never produces floating-point noise for a decimal step", () => {
    expect(snapToStep(0.3, 0, 1, 0.1)).toBe(0.3);
    expect(Number.isFinite(snapToStep(0.3, 0, 1, 0.1))).toBe(true);
  });

  it("handles an integer step with 0 decimal precision", () => {
    expect(snapToStep(17.4, 0, 100, 5)).toBe(15);
  });
});

describe("sliderKeyValue (Task 26 commit 2 - Slider keyboard a11y)", () => {
  it("moves by one step on the arrow keys", () => {
    expect(sliderKeyValue("ArrowRight", 0.5, 0, 1, 0.1)).toBeCloseTo(0.6, 10);
    expect(sliderKeyValue("ArrowUp", 0.5, 0, 1, 0.1)).toBeCloseTo(0.6, 10);
    expect(sliderKeyValue("ArrowLeft", 0.5, 0, 1, 0.1)).toBeCloseTo(0.4, 10);
    expect(sliderKeyValue("ArrowDown", 0.5, 0, 1, 0.1)).toBeCloseTo(0.4, 10);
  });

  it("moves by 10x step on PageUp/PageDown", () => {
    expect(sliderKeyValue("PageUp", 0.2, 0, 1, 0.1)).toBeCloseTo(1, 10);
    expect(sliderKeyValue("PageDown", 0.8, 0, 1, 0.1)).toBeCloseTo(0, 10);
  });

  it("jumps to the bounds on Home/End", () => {
    expect(sliderKeyValue("Home", 0.5, 0, 1, 0.1)).toBe(0);
    expect(sliderKeyValue("End", 0.5, 0, 1, 0.1)).toBe(1);
  });

  it("clamps at the bounds instead of overshooting", () => {
    expect(sliderKeyValue("ArrowRight", 0.95, 0, 1, 0.1)).toBe(1);
    expect(sliderKeyValue("ArrowLeft", 0.05, 0, 1, 0.1)).toBe(0);
  });

  it("returns null for keys it doesn't handle", () => {
    expect(sliderKeyValue("Tab", 0.5, 0, 1, 0.1)).toBeNull();
    expect(sliderKeyValue("a", 0.5, 0, 1, 0.1)).toBeNull();
  });
});
