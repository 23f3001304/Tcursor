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

// The one inspector shape: every kind reads what it is (PanelHeader) -> when it is (Timing) ->
// how it looks -> how it moves -> the destructive action. These pin the SECTION HEADINGS IN DOM
// ORDER, which is the only part of that shape a later edit can silently reorder, plus the preset
// rows, whose whole job is to apply an op and move the active segment.
const ops: EditOp[] = [];
const apply = async (op: EditOp) => { ops.push(op); return null; };
const noop = () => {};

const ZOOM: Zoom = { id: "z1", start_ms: 1000, end_ms: 3600, target: "cursor", scale: 2.2, easing: "smooth", zoom_in_ms: 350, zoom_out_ms: 450, layer: 0 };
const FX: EffectRegion = { id: "f1", kind: "spotlight", start_ms: 0, end_ms: 2000, fade_in_ms: 200, fade_out_ms: 200, layer: 0 };
const SEG: LayoutSeg = { id: "l1", start_ms: 0, end_ms: 4000, layout: "camera", transition_ms: 300, easing: "smooth", transition_out_ms: 0, easing_out: "smooth" };
const MOVE: CameraMove = { id: "m1", t_ms: 1200, x: 0.5, y: 0.5, size: 0.3, easing: "smooth" };
const CUT: Cut = { id: "c1", start_ms: 1000, end_ms: 2500 };
const SPEED: Speed = { id: "s1", start_ms: 0, end_ms: 2000, factor: 2 };
const SETTINGS = { clickfx: { spotlight_dim: 0.9, spotlight_radius: 0.15, spotlight_feather: 0.08, spotlight_dim_camera: true } } as unknown as Settings;

const rect = (x: number): PanelRectDto => ({ rect: [x, 0, 0.5, 0.5], radius: 0, alpha: 1, ring_px: 0, ring_color: [0, 0, 0] });
const preset = (x: number) => ({ screen: rect(x), cam: rect(x + 0.1), arrangement: { screen: { cx: 0.5, cy: 0.5, size: 0.5 }, cam: { cx: 0.8, cy: 0.8, size: 0.2 } } });
const PRESETS = { screen: preset(0), camera: preset(0.1), presenter: preset(0.2), screen_only: preset(0.3), camera_only: preset(0.4), segs: [], inset_w: 0.9 } as unknown as LayoutPresets;

let root: Root, container: HTMLDivElement;
const show = (node: ReactNode) => act(() => { root.render(node); });
const sections = () => [...container.querySelectorAll(".e-isec-head h3")].map((e) => e.textContent);
const segs = (group: string) => [...container.querySelectorAll<HTMLButtonElement>(`[aria-label="${group}"] .e-preseg-b`)];
const active = (group: string) => segs(group).filter((b) => b.className.includes("on")).map((b) => b.textContent);

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops.length = 0;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("one inspector shape (sections in DOM order)", () => {
  it("Zoom reads timing, framing, webcam, feel", () => {
    show(<ZoomInspector zoom={ZOOM} dur={10_000} onApply={apply} onClose={noop} aimMode={false}
      moveMode={false} onAimMode={noop} timeMsRef={{ current: 0 }} onSeek={noop} />);
    expect(sections()).toEqual(["Timing", "Framing", "Webcam during zoom", "Feel"]);
    expect(container.querySelector(".e-phead .e-lede")?.textContent).toBe("0:01.0 to 0:03.6, 2.6 s");
    expect(container.querySelector(".e-del")?.textContent).toContain("Delete zoom");
  });

  it("Spotlight reads timing, look, fades", () => {
    show(<EffectInspector effect={FX} dur={10_000} settings={SETTINGS} onApply={apply} onDimCamera={noop} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Look", "Fades"]);
  });

  it("Layout reads timing, composition, transition", () => {
    show(<LayoutInspector seg={SEG} dur={10_000} presets={PRESETS} arrangeOn={false} onApply={apply} onArrange={noop} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Composition", "Transition"]);
  });

  it("Camera Move reads timing, placement, transition", () => {
    show(<CameraMoveInspector move={MOVE} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Placement", "Transition"]);
    expect(container.querySelector(".e-phead .e-lede")?.textContent).toBe("Keyframe at 0:01.2");
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

  it("puts Remove last in every inspector", () => {
    show(<ZoomInspector zoom={ZOOM} dur={10_000} onApply={apply} onClose={noop} aimMode={false}
      moveMode={false} onAimMode={noop} timeMsRef={{ current: 0 }} onSeek={noop} />);
    const panel = container.querySelector(".e-insp");
    expect(panel?.lastElementChild?.className).toContain("e-del");
  });
});

describe("preset rows apply an op and move the active segment", () => {
  it("Zoom's Feel row starts on the matched preset and applies all four fields", () => {
    show(<ZoomInspector zoom={ZOOM} dur={10_000} onApply={apply} onClose={noop} aimMode={false}
      moveMode={false} onAimMode={noop} timeMsRef={{ current: 0 }} onSeek={noop} />);
    expect(active("Feel")).toEqual(["Balanced"]);           // ZOOM is exactly the Balanced preset
    act(() => { segs("Feel")[2].click(); });                // Punchy
    expect(ops).toEqual([{ op: "update_zoom", id: "z1", scale: 2.8, zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" }]);
    show(<ZoomInspector zoom={{ ...ZOOM, scale: 2.8, zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" }} dur={10_000}
      onApply={apply} onClose={noop} aimMode={false} moveMode={false} onAimMode={noop} timeMsRef={{ current: 0 }} onSeek={noop} />);
    expect(active("Feel")).toEqual(["Punchy"]);
  });

  it("Layout's preset row draws a mini-canvas per preset and switches layout plus arrangement", () => {
    show(<LayoutInspector seg={SEG} dur={10_000} presets={PRESETS} arrangeOn={false} onApply={apply} onArrange={noop} onClose={noop} />);
    expect(active("Start from")).toEqual(["Camera"]);
    expect(container.querySelectorAll('[aria-label="Start from"] .e-preseg-thumb').length).toBe(4);
    act(() => { segs("Start from")[1].click(); });          // Presenter
    expect(ops[0]).toEqual({ op: "update_layout_seg", id: "l1", layout: "presenter" });
  });
});
