import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { CameraMove, Cut, EditOp, EffectRegion, LayoutSeg, Speed, Zoom } from "../../lib/edit";
import type { LayoutPresets, PanelRectDto } from "../../lib/ipc";
import type { Settings } from "../../hud/settings/settings";
import { ZoomInspector } from "./ZoomInspector";
import { EffectInspector } from "./EffectInspector";
import { LayoutInspector } from "./LayoutInspector";
import { CameraMoveInspector } from "./CameraMoveInspector";
import { CutInspector } from "./CutInspector";
import { SpeedInspector } from "./SpeedInspector";

// The one inspector shape: a header (the lane dot, the name, the span, a quiet Delete) -> the
// kind's sections. These pin the SECTION HEADINGS IN DOM ORDER, which is the only part of that
// shape a later edit can silently reorder, plus the header's own parts and the preset rows, whose
// whole job is to apply an op and move the active segment.
const ops: EditOp[] = [];
const apply = async (op: EditOp) => { ops.push(op); return null; };
const noop = () => {};

const ZOOM: Zoom = { id: "z1", start_ms: 1000, end_ms: 3600, target: "cursor", scale: 2.2, easing: "smooth", zoom_in_ms: 350, zoom_out_ms: 450, layer: 0 };
const FX: EffectRegion = { id: "f1", kind: "spotlight", start_ms: 0, end_ms: 2000, fade_in_ms: 200, fade_out_ms: 200, layer: 0 };
const SEG: LayoutSeg = { id: "l1", start_ms: 0, end_ms: 4000, layout: "camera", transition_ms: 300, easing: "smooth", transition_out_ms: 0, easing_out: "smooth" };
const MOVE: CameraMove = { id: "m1", t_ms: 1200, x: 0.5, y: 0.5, size: 0.3, easing: "smooth", shape: "layout", roundness: 0.12 };
const CUT: Cut = { id: "c1", start_ms: 1000, end_ms: 2500 };
const SPEED: Speed = { id: "s1", start_ms: 0, end_ms: 2000, factor: 2 };
const SETTINGS = { clickfx: { spotlight_dim: 0.9, spotlight_radius: 0.15, spotlight_feather: 0.08, spotlight_dim_camera: true } } as unknown as Settings;

const rect = (x: number): PanelRectDto => ({ rect: [x, 0, 0.5, 0.5], radius: 0, alpha: 1, ring_px: 0, ring_color: [0, 0, 0] });
const preset = (x: number) => ({ screen: rect(x), cam: rect(x + 0.1), arrangement: { screen: { cx: 0.5, cy: 0.5, size: 0.5 }, cam: { cx: 0.8, cy: 0.8, size: 0.2 } } });
const PRESETS = { screen: preset(0), camera: preset(0.1), presenter: preset(0.2), screen_only: preset(0.3), camera_only: preset(0.4), segs: [], inset_w: 0.9 } as unknown as LayoutPresets;

let root: Root, container: HTMLDivElement;
const show = (node: ReactNode) => act(() => { root.render(node); });
const zoomAt = (z: Partial<Zoom> = {}) => show(<ZoomInspector zoom={{ ...ZOOM, ...z }} dur={10_000} onApply={apply}
  onClose={noop} aimMode={false} moveMode={false} onAimMode={noop} timeMsRef={{ current: 0 }} onSeek={noop} />);
const sections = () => [...container.querySelectorAll(".e-isec-head h3")].map((e) => e.textContent);
const segs = (group: string) => [...container.querySelectorAll<HTMLButtonElement>(`[aria-label="${group}"] .e-preseg-b`)];
const active = (group: string) => segs(group).filter((b) => b.className.includes("on")).map((b) => b.textContent);
const range = () => container.querySelector(".e-ihead-range")?.textContent;
const del = () => container.querySelector<HTMLButtonElement>(".e-ihicon.del");

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops.length = 0;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("one inspector shape (sections in DOM order)", () => {
  it("Zoom reads framing, timing, feel, webcam", () => {
    zoomAt();
    expect(sections()).toEqual(["Framing", "Timing", "Feel", "Webcam during zoom"]);
    expect(range()).toBe("1.00s to 3.60s");
    expect(del()?.getAttribute("aria-label")).toBe("Delete zoom");
  });

  it("Spotlight reads timing, look, fades", () => {
    show(<EffectInspector effect={FX} dur={10_000} settings={SETTINGS} onApply={apply} onDimCamera={noop} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Look", "Fades"]);
    expect(range()).toBe("0.00s to 2.00s");
  });

  it("Layout reads timing, composition, transition", () => {
    show(<LayoutInspector seg={SEG} dur={10_000} presets={PRESETS} arrangeOn={false} onApply={apply} onArrange={noop} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Composition", "Transition"]);
  });

  it("Camera Move reads timing, placement, shape, transition", () => {
    show(<CameraMoveInspector move={MOVE} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Placement", "Shape", "Transition"]);
    expect(range()).toBe("Keyframe at 1.20s");
  });

  it("Cut reads timing only, and says how much it removes", () => {
    show(<CutInspector cut={CUT} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing"]);
    expect(container.querySelector(".e-isec-val")?.textContent).toBe("removes 1.5 s");
  });

  it("Speed reads timing, rate", () => {
    show(<SpeedInspector speed={SPEED} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Rate"]);
  });

  it("puts Delete in the header of every inspector, and a section last", () => {
    zoomAt();
    const panel = container.querySelector(".e-insp");
    expect(panel?.firstElementChild?.className).toContain("e-ihead");
    expect(panel?.lastElementChild?.className).toContain("e-isec");
    act(() => { del()?.click(); });
    expect(ops).toEqual([{ op: "remove_zoom", id: "z1" }]);
  });
});

describe("Zoom's Framing hero and grouped Timing row", () => {
  it("sets the scale as the section's hero value", () => {
    zoomAt({ scale: 2.8 });
    expect(container.querySelector(".e-ihero-v")?.textContent).toBe("2.8x");
    expect(container.querySelector(".e-ihero-l")?.textContent).toBe("Scale");
  });

  it("groups start, end, in and out as one row of four value fields", () => {
    zoomAt();
    const cells = [...container.querySelectorAll('[aria-label="Timing"] .e-ival')];
    expect(cells.map((c) => c.querySelector(".e-ival-l")?.textContent)).toEqual(["Start", "End", "In", "Out"]);
    expect(cells.map((c) => c.querySelector(".e-ival-v")?.textContent).slice(0, 1).concat(cells.slice(2).map((c) => c.querySelector(".e-ival-v")?.textContent))).toEqual(["1.00s", "0.35s", "0.45s"]);
  });

  it("steps one cell by 0.05s without touching the other three", () => {
    zoomAt();
    act(() => { container.querySelector<HTMLButtonElement>('[aria-label="More In"]')?.click(); });
    expect(ops).toEqual([{ op: "update_zoom", id: "z1", zoom_in_ms: 400 }]);
  });

  it("clamps a cell at its own floor rather than going negative", () => {
    zoomAt({ zoom_in_ms: 0 });
    expect(container.querySelector<HTMLButtonElement>('[aria-label="Less In"]')?.disabled).toBe(true);
  });
});

describe("preset rows apply an op and move the active segment", () => {
  it("Zoom's Feel row starts on the matched preset and applies the three fields a feel owns", () => {
    zoomAt();                                              // ZOOM is exactly the Balanced preset
    expect(active("Feel")).toEqual(["Balanced"]);
    act(() => { segs("Feel")[2].click(); });                // Punchy
    expect(ops).toEqual([{ op: "update_zoom", id: "z1", zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" }]);
    zoomAt({ zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" });
    expect(active("Feel")).toEqual(["Punchy"]);
  });

  it("lights no plane at all when the values match no preset", () => {
    zoomAt({ zoom_in_ms: 123 });
    expect(active("Feel")).toEqual([]);
    expect(container.querySelectorAll(".e-isec-val")[1]?.textContent).toBe("Custom");
  });

  it("keeps the curve editor and the spring sliders under the quiet Custom disclosure", () => {
    zoomAt();
    const more = container.querySelector<HTMLButtonElement>(".e-more-btn");
    expect(more?.textContent).toContain("Custom");
    expect(more?.getAttribute("aria-expanded")).toBe("false");
    expect(container.querySelector(".e-curve-box")).toBeNull();
  });

  it("Layout's preset row draws a mini-canvas per preset and switches layout plus arrangement", () => {
    show(<LayoutInspector seg={SEG} dur={10_000} presets={PRESETS} arrangeOn={false} onApply={apply} onArrange={noop} onClose={noop} />);
    expect(active("Start from")).toEqual(["Camera"]);
    expect(container.querySelectorAll('[aria-label="Start from"] .e-preseg-thumb').length).toBe(4);
    act(() => { segs("Start from")[1].click(); });          // Presenter
    expect(ops[0]).toEqual({ op: "update_layout_seg", id: "l1", layout: "presenter" });
  });
});
