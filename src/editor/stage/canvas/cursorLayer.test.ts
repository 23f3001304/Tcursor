import { describe, it, expect, vi, beforeEach } from "vitest";

const sprite = vi.fn();
vi.mock("../cursor/cursorPreview", () => ({ drawCursorSprite: (...a: unknown[]) => sprite(...a) }));

import { drawCursorLayer } from "./cursorLayer";
import type { PreviewGeom } from "./previewCanvas";
import type { DrawCursor } from "../cursor/cursorPreview";

const noopCtx = () =>
  new Proxy({}, { get: () => () => undefined, set: () => true }) as unknown as CanvasRenderingContext2D;

const untouchable = new Proxy(
  {},
  {
    get(_t, p) {
      throw new Error(`drawCursorLayer touched ctx.${String(p)} on a no-op path`);
    },
  },
) as unknown as CanvasRenderingContext2D;

const CAM = { scale: 1, cx: 0.5, cy: 0.5, curx: 0.25, cury: 0.5 };

const geom = (over: Partial<PreviewGeom> = {}): PreviewGeom => ({
  panel: [0, 0, 400, 200],
  crop: [0, 0, 400, 200],
  src: [0, 0, 1, 1],
  screenAlpha: 1,
  hasVideo: true,
  ...over,
});

const cursorStub = { captured: null } as unknown as DrawCursor;

beforeEach(() => sprite.mockReset());

describe("drawCursorLayer", () => {
  it("never touches ctx with no cursor, no video, or a hidden screen panel", () => {
    drawCursorLayer(untouchable, 400, 200, CAM, null, [], 0, 1, geom());
    drawCursorLayer(untouchable, 400, 200, CAM, cursorStub, [], 0, 1, geom({ hasVideo: false }));
    drawCursorLayer(untouchable, 400, 200, CAM, cursorStub, [], 0, 1, geom({ screenAlpha: 0.4 }));
    expect(sprite).not.toHaveBeenCalled();
  });

  it("places the sprite at the cursor's projected canvas point", () => {
    drawCursorLayer(noopCtx(), 400, 200, CAM, cursorStub, [], 0, 1, geom());
    expect(sprite).toHaveBeenCalledTimes(1);
    expect(sprite.mock.calls[0][1]).toEqual([100, 100]);
  });
});
