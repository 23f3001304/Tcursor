import { act, type ReactNode } from "react";
import { beforeEach, afterEach } from "vitest";
import { createRoot, type Root } from "react-dom/client";
import type { CameraMove, Cut, EditOp, EffectRegion, LayoutSeg, Speed, Zoom } from "../../shared/edit";
import type { LayoutPresets, PanelRectDto } from "../../shared/ipc";
import type { Settings } from "../../hud/settings/settings";
import { ZoomInspector } from "./ZoomInspector";
import { LayoutInspector } from "./LayoutInspector";

export const ops: EditOp[] = [];
export const apply = async (op: EditOp) => {
  ops.push(op);
  return null;
};
export const noop = () => {};

export const ZOOM: Zoom = {
  id: "z1",
  start_ms: 1000,
  end_ms: 3600,
  target: "cursor",
  scale: 2.2,
  easing: "smooth",
  zoom_in_ms: 350,
  zoom_out_ms: 450,
  layer: 0,
};
export const FX: EffectRegion = {
  id: "f1",
  kind: "spotlight",
  start_ms: 0,
  end_ms: 2000,
  fade_in_ms: 200,
  fade_out_ms: 200,
  layer: 0,
};
export const SEG: LayoutSeg = {
  id: "l1",
  start_ms: 0,
  end_ms: 4000,
  layout: "camera",
  transition_ms: 300,
  easing: "smooth",
  transition_out_ms: 0,
  easing_out: "smooth",
};
export const MOVE: CameraMove = {
  id: "m1",
  t_ms: 1200,
  x: 0.5,
  y: 0.5,
  size: 0.3,
  easing: "smooth",
  shape: "layout",
  roundness: 0.12,
};
export const CUT: Cut = { id: "c1", start_ms: 1000, end_ms: 2500 };
export const SPEED: Speed = { id: "s1", start_ms: 0, end_ms: 2000, factor: 2 };
export const SETTINGS = {
  clickfx: {
    spotlight_dim: 0.9,
    spotlight_radius: 0.15,
    spotlight_feather: 0.08,
    spotlight_dim_camera: true,
  },
} as unknown as Settings;

const rect = (x: number): PanelRectDto => ({
  rect: [x, 0, 0.5, 0.5],
  radius: 0,
  alpha: 1,
  ring_px: 0,
  ring_color: [0, 0, 0],
});
const preset = (x: number) => ({
  screen: rect(x),
  cam: rect(x + 0.1),
  arrangement: { screen: { cx: 0.5, cy: 0.5, size: 0.5 }, cam: { cx: 0.8, cy: 0.8, size: 0.2 } },
});
export const PRESETS = {
  screen: preset(0),
  camera: preset(0.1),
  presenter: preset(0.2),
  screen_only: preset(0.3),
  camera_only: preset(0.4),
  segs: [],
  inset_w: 0.9,
} as unknown as LayoutPresets;

let root: Root, container: HTMLDivElement;

/** Wires the per-test React root. Call once at the top of a test file's module scope. */
export function useInspectorDom(): void {
  beforeEach(() => {
    (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    ops.length = 0;
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
  });
}

export const show = (node: ReactNode) =>
  act(() => {
    root.render(node);
  });

export const q = <T extends Element>(sel: string) => container.querySelector<T>(sel);
export const qa = <T extends Element>(sel: string) => [...container.querySelectorAll<T>(sel)];

export const zoomAt = (z: Partial<Zoom> = {}) =>
  show(
    <ZoomInspector
      zoom={{ ...ZOOM, ...z }}
      zooms={[]}
      dur={10_000}
      onApply={apply}
      onClose={noop}
      aimMode={false}
      moveMode={false}
      onAimMode={noop}
      timeMsRef={{ current: 0 }}
      onSeek={noop}
    />,
  );

export const layoutAt = () =>
  show(
    <LayoutInspector
      seg={SEG}
      segs={[]}
      dur={10_000}
      presets={PRESETS}
      arrangeOn={false}
      onApply={apply}
      onArrange={noop}
      onClose={noop}
    />,
  );

export const sections = () => qa(".e-isec-head h3").map((e) => e.textContent);
export const segs = (group: string) => qa<HTMLButtonElement>(`[aria-label="${group}"] .e-preseg-b`);
export const active = (group: string) =>
  segs(group)
    .filter((b) => b.className.includes("on"))
    .map((b) => b.textContent);
export const motionSegs = () => qa<HTMLButtonElement>('[aria-label="Motion preset"] .e-segment');
export const motionActive = () =>
  motionSegs()
    .filter((b) => b.className.includes("on"))
    .map((b) => b.textContent);
export const range = () => q(".e-ihead-range")?.textContent;
export const del = () => q<HTMLButtonElement>(".e-ihicon.del");
