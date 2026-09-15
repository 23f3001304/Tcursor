import { describe, it, expect } from "vitest";
import { assetFileName, assetKindOf, assetSubtitle, thumbRel } from "./backgroundAsset";

describe("assetFileName", () => {
  it("reads the file name off the relative path", () => {
    expect(assetFileName("background/my clip.mp4")).toBe("my clip.mp4");
    expect(assetFileName("hero.png")).toBe("hero.png");
    expect(assetFileName("")).toBe("");
  });
});

describe("assetKindOf", () => {
  it("mirrors the Rust extension table, case included", () => {
    expect(assetKindOf("background/a.GIF")).toBe("video");
    expect(assetKindOf("background/a.mp4")).toBe("video");
    expect(assetKindOf("background/a.webp")).toBe("image");
    expect(assetKindOf("background/a.jpeg")).toBe("image");
    expect(assetKindOf("background/a.txt")).toBeNull();
    expect(assetKindOf("")).toBeNull();
  });
});

describe("assetSubtitle", () => {
  it("says what the asset is, with a duration only when there is one", () => {
    expect(
      assetSubtitle({
        rel_path: "background/a.mp4",
        kind: "video",
        width: 1920,
        height: 1080,
        duration_ms: 4200,
      }),
    ).toBe("Video 1920x1080, 4.2s");
    expect(
      assetSubtitle({
        rel_path: "background/a.png",
        kind: "image",
        width: 2560,
        height: 1440,
        duration_ms: null,
      }),
    ).toBe("Image 2560x1440");
  });

  it("says plainly when the file is gone, and stays quiet while the probe is in flight", () => {
    expect(assetSubtitle(null)).toBe("File missing");
    expect(assetSubtitle(undefined)).toBe("");
  });

  it("drops a dimension it could not probe rather than printing 0x0", () => {
    expect(
      assetSubtitle({ rel_path: "background/a.mp4", kind: "video", width: 0, height: 0, duration_ms: 1000 }),
    ).toBe("Video 1.0s");
  });
});

describe("thumbRel", () => {
  it("derives the thumbnail path exactly the way Rust does", () => {
    expect(thumbRel("background/a.mp4")).toBe("background/.thumbs/a.mp4.jpg");
    expect(thumbRel("a.png")).toBe(".thumbs/a.png.jpg");
  });
});
