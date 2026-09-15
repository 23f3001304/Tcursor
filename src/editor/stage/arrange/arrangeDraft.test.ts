// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import type { LayoutSeg, PanelPose } from "../../../shared/edit";
import type { LayoutPresets, PanelRectDto } from "../../../shared/ipc";
import {
  draftPanels,
  poseOfRect,
  setArrangementOp,
  withDraftSeg,
  type Panels,
  type Rect,
} from "./arrangeMath";

const panel = (rect: Rect, over: Partial<PanelRectDto> = {}): PanelRectDto => ({
  rect,
  radius: 0.02,
  alpha: 1,
  ring_px: 0.004,
  ring_color: [1, 2, 3],
  ...over,
});
const RECT: Rect = [0.25, 0.25, 0.5, 0.5];
const CAM: Rect = [0.7, 0.7, 0.2, 0.2];
const base = (): Panels => ({ screen: panel(RECT), cam: panel(CAM) });
const seg = (over: Partial<LayoutSeg> = {}): LayoutSeg => ({
  id: "s1",
  start_ms: 0,
  end_ms: 1000,
  layout: "camera",
  transition_ms: 0,
  easing: "smooth",
  transition_out_ms: 0,
  easing_out: "smooth",
  ...over,
});
const pose = (cx: number, cy: number, size: number): PanelPose => ({ cx, cy, size });

describe("draftPanels", () => {
  const moved: Rect = [0.3, 0.3, 0.25, 0.25];

  it("replaces only the dragged panel's rect and forces it visible", () => {
    const out = draftPanels({ screen: panel(RECT, { alpha: 0 }), cam: panel(CAM) }, "screen", moved);
    expect(out.screen.rect).toBe(moved);
    expect(out.screen.alpha).toBe(1);
    expect(out.cam).toEqual(panel(CAM));
  });

  it("leaves the SCREEN radius alone (it is a fraction of canvas height, not of the panel)", () => {
    expect(draftPanels(base(), "screen", moved).screen.radius).toBe(0.02);
  });

  it("scales the CAM radius and ring by the height ratio, so a circle stays round mid-drag", () => {
    const out = draftPanels(base(), "cam", [0.5, 0.5, 0.4, 0.4]);
    expect(out.cam.radius).toBeCloseTo(0.04, 9);
    expect(out.cam.ring_px).toBeCloseTo(0.008, 9);
  });
});

describe("withDraftSeg", () => {
  const presets = { segs: [{ id: "other", screen: null, cam: null }] } as unknown as LayoutPresets;

  it("appends an entry for a segment that has none yet, leaving the others alone", () => {
    const out = withDraftSeg(presets, "s1", base());
    expect(out.segs).toHaveLength(2);
    expect(out.segs[0]).toBe(presets.segs[0]);
    expect(out.segs[1].id).toBe("s1");
    expect(presets.segs).toHaveLength(1);
  });

  it("replaces an existing entry by id rather than appending a duplicate", () => {
    const seeded = { segs: [{ id: "s1", screen: panel(CAM), cam: panel(CAM) }] } as unknown as LayoutPresets;
    const out = withDraftSeg(seeded, "s1", base());
    expect(out.segs).toHaveLength(1);
    expect(out.segs[0].screen).toEqual(panel(RECT));
  });
});

describe("setArrangementOp", () => {
  it("touches only the dragged panel once the segment already has an arrangement", () => {
    const s = seg({ arrangement: { screen: pose(0.5, 0.5, 0.5), cam: null } });
    const op = setArrangementOp(s, base(), "screen", pose(0.4, 0.4, 0.6));
    expect(op).toEqual({ op: "set_arrangement", id: "s1", screen: { cx: 0.4, cy: 0.4, size: 0.6 } });
    expect("cam" in op).toBe(false);
  });

  it("fills the untouched panel in on a FIRST write (the backend bases that off 'both hidden')", () => {
    const op = setArrangementOp(seg(), base(), "cam", pose(0.8, 0.8, 0.3)) as Record<string, unknown>;
    expect(op.cam).toEqual({ cx: 0.8, cy: 0.8, size: 0.3 });
    expect(op.screen).toEqual(poseOfRect(RECT));
  });

  it("keeps a panel the provenance preset HIDES hidden, as an explicit null", () => {
    const hidden: Panels = { screen: panel(RECT, { alpha: 0 }), cam: panel(CAM) };
    const op = setArrangementOp(seg(), hidden, "cam", pose(0.5, 0.5, 0.3)) as Record<string, unknown>;
    expect(op.screen).toBeNull();
  });

  it("hiding the cam on a bare segment still carries the screen, so the write is never 'hide both'", () => {
    const op = setArrangementOp(seg(), base(), "cam", null) as Record<string, unknown>;
    expect(op.cam).toBeNull();
    expect(op.screen).toEqual(poseOfRect(RECT));
  });
});
