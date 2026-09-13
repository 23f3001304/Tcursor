import { describe, it, expect } from "vitest";
import { panelFactor, panelClipRect, contentScale } from "./cursorPanel";

describe("contentScale", () => {
  // The TS mirror of Rust `captured::content_scale`: `panel * insetPx` IS the screen panel's own
  // width, so this is "canvas px per source px" - the captured cursor's size relative to the
  // screen content. The Rust test pins the same numbers.
  it("leaves a source the same size as its panel untouched", () => {
    expect(contentScale(1, 1920, 1920)).toBe(1);
  });
  it("halves a 2x source (a 4K take on a 1080p canvas)", () => {
    expect(contentScale(1, 1920, 3840)).toBe(0.5);
  });
  it("grows an upscaled source with its content", () => {
    expect(contentScale(1, 1920, 960)).toBe(2);
  });
  it("applies a shrunk panel once, not squared", () => {
    expect(contentScale(0.5, 1920, 1920)).toBe(0.5);
    expect(contentScale(0.5, 1920, 3840)).toBe(0.25);
  });
  it("falls back to the panel factor alone when the source width is unknown", () => {
    // src_w 0 = the backend could not probe the video; better the old panel-only size than a
    // cursor collapsed to nothing by a divide-by-zero.
    expect(contentScale(0.7, 1920, 0)).toBe(0.7);
    expect(contentScale(1, 1920, -5)).toBe(1);
  });
});

describe("panelFactor", () => {
  it("gives 1.0 for a full-frame panel (screenW == insetW)", () => {
    expect(panelFactor(0.8, 0.8)).toBeCloseTo(1.0);
  });
  it("gives 0.5 for a panel half the reference width", () => {
    expect(panelFactor(0.4, 0.8)).toBeCloseTo(0.5);
  });
  it("floors at 0.1 for a much narrower panel", () => {
    expect(panelFactor(0.01, 0.8)).toBe(0.1);
  });
  it("caps at 1.0 for a panel wider than the reference", () => {
    expect(panelFactor(1.0, 0.8)).toBe(1.0);
  });
  it("never divides by zero on a degenerate insetW", () => {
    expect(Number.isFinite(panelFactor(0.5, 0))).toBe(true);
    expect(panelFactor(0.5, 0)).toBe(1.0); // clamped up like an oversized ratio
  });
});

describe("panelClipRect", () => {
  const IDENTITY = { cx0: 0, cy0: 0, cw: 1920, ch: 1080 };

  it("gives a clip equal to the frame for a full-frame panel under an identity crop", () => {
    const clip = panelClipRect({ x: 0, y: 0, w: 1920, h: 1080 }, IDENTITY, 1920, 1080);
    expect(clip).toEqual([0, 0, 1920, 1080]);
  });

  it("clips a smaller panel to its own rect under an identity crop", () => {
    const clip = panelClipRect({ x: 100, y: 50, w: 800, h: 600 }, IDENTITY, 1920, 1080);
    expect(clip).toEqual([100, 50, 900, 650]);
  });

  it("follows the zoom crop: a 2x zoom centered on the panel doubles it and re-centers", () => {
    // Same crop math previewCanvas.ts uses: scale 2 over a 1920x1080 canvas crops a
    // 960x540 window; centering that window on the panel's own center (960,540) puts the
    // crop's top-left at (480,270).
    const crop = { cx0: 480, cy0: 270, cw: 960, ch: 540 };
    const clip = panelClipRect({ x: 0, y: 0, w: 1920, h: 1080 }, crop, 1920, 1080);
    // The whole (now off-canvas-bounds) frame projects to [-960,-540, 2880,1620], clamped
    // to the canvas - i.e. the zoomed-in panel still covers the full visible canvas.
    expect(clip).toEqual([0, 0, 1920, 1080]);
  });

  it("leaves an inverted (x0 past x1) clip, unnormalized, when the panel is entirely outside the crop", () => {
    // Mirrors cursorset.rs's project_rect: no reordering here - the caller's no-op guard
    // (ox_start >= ox_end in blit, the same check drawCursorSprite carries) is what makes
    // this safe, not a normalized rect.
    const crop = { cx0: 0, cy0: 0, cw: 100, ch: 100 }; // crop window is the top-left 100x100
    const clip = panelClipRect({ x: 500, y: 500, w: 200, h: 200 }, crop, 1920, 1080);
    expect(clip[2]).toBeLessThan(clip[0]);
  });
});
