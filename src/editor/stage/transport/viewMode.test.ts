// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { stageFrameStyle } from "./viewMode";

describe("stageFrameStyle", () => {
  it("leaves Fit to the CSS aspect-ratio sizer, measured or not", () => {
    expect(stageFrameStyle("fit", 1280, 720, 900, 500)).toEqual({ aspectRatio: "1280 / 720" });
  });

  it("falls back to Fit until the stage area has been measured", () => {
    expect(stageFrameStyle("fill", 1280, 720, 0, 0)).toEqual({ aspectRatio: "1280 / 720" });
    expect(stageFrameStyle("native", 1280, 720, 900, 0)).toEqual({ aspectRatio: "1280 / 720" });
  });

  it("covers the area in Fill, overflowing the axis that does not fit", () => {
    expect(stageFrameStyle("fill", 1280, 720, 800, 600)).toEqual({
      width: 1067,
      height: 600,
      maxWidth: "none",
      maxHeight: "none",
    });
    expect(stageFrameStyle("fill", 1280, 720, 1600, 600)).toEqual({
      width: 1600,
      height: 900,
      maxWidth: "none",
      maxHeight: "none",
    });
  });

  it("shows source pixels at 100%, and never upscales past them", () => {
    expect(stageFrameStyle("native", 1280, 720, 1600, 900)).toEqual({
      width: 1280,
      height: 720,
      maxWidth: "none",
      maxHeight: "none",
    });
  });

  it("but does scale 100% down to fit an area smaller than the canvas", () => {
    expect(stageFrameStyle("native", 1280, 720, 800, 600)).toEqual({
      width: 800,
      height: 450,
      maxWidth: "none",
      maxHeight: "none",
    });
    expect(stageFrameStyle("native", 1280, 720, 1600, 400)).toEqual({
      width: 711,
      height: 400,
      maxWidth: "none",
      maxHeight: "none",
    });
  });
});
