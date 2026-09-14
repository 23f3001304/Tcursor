import { describe, it, expect } from "vitest";
import { EDGE_MARGIN, placeBeside, placeStacked, placeStackedCentred, portalHost } from "./popoverPlace";

const VW = 1000, VH = 600;
const rect = (left: number, top: number, width = 40, height = 40) => ({ left, top, width, height });

describe("placeBeside", () => {
  it("sits to the right of the anchor, centred on it", () => {
    const p = placeBeside(rect(56, 200), 120, 26, VW, VH);
    expect(p).toEqual({ left: 56 + 40 + 8, top: 200 + 20 - 13, flipped: false });
  });

  it("flips to the left when the right side would leave the window", () => {
    const p = placeBeside(rect(900, 200), 120, 26, VW, VH);
    expect(p.flipped).toBe(true);
    expect(p.left).toBe(900 - 8 - 120);
  });

  it("stays inside the window when the label is wider than the window itself", () => {
    const p = placeBeside(rect(4, 4), 1200, 26, VW, VH);
    expect(p.left).toBe(EDGE_MARGIN);
  });

  it("keeps a label off the top edge when its anchor is right against it", () => {
    expect(placeBeside(rect(56, 0, 40, 20), 120, 26, VW, VH).top).toBe(EDGE_MARGIN);
  });
});

describe("placeStacked", () => {
  it("opens below the anchor when there is room", () => {
    const p = placeStacked(rect(100, 100, 200, 34), 200, 216, VW, VH);
    expect(p).toEqual({ left: 100, top: 138, flipped: false });
  });

  it("flips above when the menu would run past the bottom of the window", () => {
    const p = placeStacked(rect(100, 520, 200, 34), 200, 216, VW, VH);
    expect(p.flipped).toBe(true);
    expect(p.top).toBe(520 - 4 - 216);
  });

  it("stays below when neither side fits, so the first row is still reachable", () => {
    const p = placeStacked(rect(100, 300, 200, 34), 200, 900, VW, VH);
    expect(p.flipped).toBe(false);
    expect(p.top).toBe(EDGE_MARGIN);
  });

  it("prefers above, and flips down when the anchor is near the top", () => {
    expect(placeStacked(rect(100, 300, 60, 32), 170, 96, VW, VH, 6, true).flipped).toBe(false);
    const down = placeStacked(rect(100, 10, 60, 32), 170, 96, VW, VH, 6, true);
    expect(down.flipped).toBe(true);
    expect(down.top).toBe(10 + 32 + 6);
  });

  it("clamps a menu whose anchor sits against the right edge", () => {
    expect(placeStacked(rect(960, 100, 30, 30), 170, 96, VW, VH).left).toBe(VW - EDGE_MARGIN - 170);
  });
});

describe("placeStackedCentred", () => {
  it("centres the layer on the anchor", () => {
    expect(placeStackedCentred(rect(400, 300, 60, 32), 170, 96, VW, VH, 6, true).left).toBe(400 + 30 - 85);
  });

  it("clamps rather than centring when the anchor is at the very start of the track", () => {
    expect(placeStackedCentred(rect(0, 300, 10, 32), 170, 96, VW, VH, 6, true).left).toBe(EDGE_MARGIN);
  });
});

describe("portalHost", () => {
  it("mounts into the editor root so the --e-* palette still inherits", () => {
    const editor = document.createElement("div");
    editor.className = "editor";
    const deep = document.createElement("span");
    editor.appendChild(deep);
    document.body.appendChild(editor);
    expect(portalHost(deep)).toBe(editor);
    editor.remove();
  });

  it("falls back to the body outside the editor", () => {
    expect(portalHost(null)).toBe(document.body);
  });
});
