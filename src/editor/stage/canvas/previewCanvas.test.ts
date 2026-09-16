import { describe, it, expect, vi, beforeEach } from "vitest";

const sprite = vi.fn();
vi.mock("../cursor/cursorPreview", () => ({ drawCursorSprite: (...a: unknown[]) => sprite(...a) }));
vi.mock("./stageBg", () => ({ drawBackground: () => {} }));

import { drawPreview } from "./previewCanvas";

const noopCtx = () =>
  new Proxy({}, { get: () => () => undefined, set: () => true }) as unknown as CanvasRenderingContext2D;

const canvasStub = (has2d: boolean) =>
  ({ width: 0, height: 0, getContext: () => (has2d ? noopCtx() : null) }) as unknown as HTMLCanvasElement;

const videoStub = (vw: number, vh: number) =>
  ({ videoWidth: vw, videoHeight: vh }) as unknown as HTMLVideoElement;

const CAM = { scale: 1, cx: 0.5, cy: 0.5, curx: 0.25, cury: 0.5 };

beforeEach(() => sprite.mockReset());

describe("drawPreview after the cursor split", () => {
  it("returns the panel and crop geometry and never draws the cursor itself", () => {
    const g = drawPreview(
      noopCtx(),
      400,
      200,
      videoStub(0, 0),
      null,
      CAM,
      null,
      null,
      0,
      canvasStub(true),
      canvasStub(true),
    );
    expect(g).not.toBeNull();
    expect(g!.hasVideo).toBe(false);
    expect(g!.crop).toEqual([0, 0, 400, 200]);
    expect(sprite).not.toHaveBeenCalled();
  });

  it("returns null when the offscreen canvas has no 2d context", () => {
    const g = drawPreview(
      noopCtx(),
      400,
      200,
      videoStub(0, 0),
      null,
      CAM,
      null,
      null,
      0,
      canvasStub(false),
      canvasStub(true),
    );
    expect(g).toBeNull();
  });
});
