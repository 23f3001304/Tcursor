import { describe, it, expect } from "vitest";
import { coverRect, loopMs, isGif, bgAssetUrl } from "./stageBg";

describe("coverRect", () => {
  it("covers the destination by cropping the long axis, never squashing", () => {
    expect(coverRect(1920, 1080, 1000, 1000)).toEqual([420, 0, 1080, 1080]);
    expect(coverRect(1080, 1920, 1000, 1000)).toEqual([0, 420, 1080, 1080]);
    expect(coverRect(16, 9, 32, 18)).toEqual([0, 0, 16, 9]);
  });

  it("is the whole source when either side is degenerate, so a not-yet-loaded video cannot NaN", () => {
    expect(coverRect(0, 0, 100, 100)).toEqual([0, 0, 0, 0]);
    expect(coverRect(100, 100, 0, 0)).toEqual([0, 0, 100, 100]);
  });
});

describe("loopMs", () => {
  it("wraps the playhead into the asset's own duration", () => {
    expect(loopMs(0, 2000)).toBe(0);
    expect(loopMs(2500, 2000)).toBe(500);
    expect(loopMs(4000, 2000)).toBe(0);
  });

  it("is 0 for a duration that is not known yet, never NaN", () => {
    expect(loopMs(1000, 0)).toBe(0);
    expect(loopMs(1000, Number.NaN)).toBe(0);
  });

  it("answers a playhead before the clip starts with frame 0, not the end of the loop", () => {
    expect(loopMs(-500, 2000)).toBe(0);
  });
});

describe("isGif", () => {
  it("decides the decode path by extension, because the model calls a GIF a video", () => {
    expect(isGif("background/loop.gif")).toBe(true);
    expect(isGif("background/LOOP.GIF")).toBe(true);
    expect(isGif("background/loop.mp4")).toBe(false);
    expect(isGif("")).toBe(false);
  });
});

describe("bgAssetUrl", () => {
  it("is empty unless the kind actually draws an asset, so no stale file is ever fetched", () => {
    const src = (p: string) => `asset://${p}`;
    expect(bgAssetUrl("C:\\rec", "background/a.mp4", "video", src)).toBe(
      "asset://C:\\rec\\background\\a.mp4",
    );
    expect(bgAssetUrl("C:\\rec", "background/a.png", "image", src)).toBe(
      "asset://C:\\rec\\background\\a.png",
    );
    expect(bgAssetUrl("C:\\rec", "background/a.mp4", "mesh", src)).toBe("");
    expect(bgAssetUrl("C:\\rec", null, "video", src)).toBe("");
  });
});
