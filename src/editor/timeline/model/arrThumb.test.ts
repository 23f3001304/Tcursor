import { describe, it, expect } from "vitest";
import { arrThumb, THUMB_W, THUMB_H } from "./arrThumb";
import { VISIBLE_ALPHA } from "../../stage/arrange/arrangeMath";
import type { PanelRectDto } from "../../../shared/ipc";
import type { ResolvedPanels } from "./layoutTrack";

const panel = (x: number, y: number, w: number, h: number, alpha = 1): PanelRectDto => ({
  rect: [x, y, w, h],
  radius: 0.02,
  alpha,
  ring_px: 0,
  ring_color: [0, 0, 0],
});

describe("arrThumb (T34 L4 - lane pill / inspector header schematic geometry)", () => {
  it("scales a full-screen preset's panels into the default 24x14 box", () => {
    const panels: ResolvedPanels = { screen: panel(0, 0, 1, 1), cam: panel(0.62, 0.62, 0.33, 0.33) };
    const t = arrThumb(panels);
    expect(t.screen).toEqual({ x: 0, y: 0, w: THUMB_W, h: THUMB_H });
    expect(t.cam).toEqual({ x: 0.62 * THUMB_W, y: 0.62 * THUMB_H, w: 0.33 * THUMB_W, h: 0.33 * THUMB_H });
  });

  it("scales a small-cam preset (Presenter-ish) at the inspector's 48x28 size, both axes independent", () => {
    const panels: ResolvedPanels = { screen: panel(0, 0, 0.8, 0.8), cam: panel(0.7, 0.7, 0.2, 0.2) };
    const t = arrThumb(panels, 48, 28);
    expect(t.screen).toEqual({ x: 0, y: 0, w: 0.8 * 48, h: 0.8 * 28 });
    expect(t.cam).toEqual({ x: 0.7 * 48, y: 0.7 * 28, w: 0.2 * 48, h: 0.2 * 28 });
  });

  it("hides a panel resolved as hidden (camera_only-style: screen hidden, cam shown)", () => {
    const panels: ResolvedPanels = {
      screen: panel(0.1, 0.1, 0.8, 0.8, 0),
      cam: panel(0.2, 0.2, 0.6, 0.6, 1),
    };
    const t = arrThumb(panels);
    expect(t.screen).toBeNull();
    expect(t.cam).not.toBeNull();
  });

  it("hides the cam symmetrically (screen_only-style: cam hidden, screen shown)", () => {
    const panels: ResolvedPanels = { screen: panel(0, 0, 1, 1, 1), cam: panel(0.7, 0.7, 0.2, 0.2, 0) };
    expect(arrThumb(panels).cam).toBeNull();
    expect(arrThumb(panels).screen).not.toBeNull();
  });

  it("draws a custom arrangement's arbitrary rect exactly - not any preset's own placement", () => {
    const panels: ResolvedPanels = { screen: panel(0.05, 0.12, 0.5, 0.6), cam: panel(0.6, 0.05, 0.35, 0.28) };
    const t = arrThumb(panels);
    expect(t.screen).toEqual({ x: 0.05 * THUMB_W, y: 0.12 * THUMB_H, w: 0.5 * THUMB_W, h: 0.6 * THUMB_H });
    expect(t.cam).toEqual({ x: 0.6 * THUMB_W, y: 0.05 * THUMB_H, w: 0.35 * THUMB_W, h: 0.28 * THUMB_H });
  });

  it("falls back to both-null when there are no resolved panels at all (presets not loaded / no segment)", () => {
    expect(arrThumb(null)).toEqual({ screen: null, cam: null });
  });

  it("treats exactly VISIBLE_ALPHA as hidden - the same boundary ArrangeOverlay/LayoutInspector use", () => {
    const panels: ResolvedPanels = { screen: panel(0, 0, 1, 1, VISIBLE_ALPHA), cam: panel(0, 0, 1, 1, 1) };
    expect(arrThumb(panels).screen).toBeNull();
    expect(arrThumb(panels).cam).not.toBeNull();
  });
});
