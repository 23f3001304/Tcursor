import { describe, it, expect } from "vitest";
import type { EffectRegion, LayoutSeg } from "../../../shared/edit";
import type { LayoutPresetDto, LayoutPresets, PreviewLayout } from "../../../shared/ipc";
import type { ZoomSettings } from "../../../hud/settings/settings";
import { camAt } from "../camera/camera";
import { activeCamDraft, frameCamLayout } from "../camera/frameCam";
import { layoutAt } from "../../timeline/model/layoutTrack";
import { maskFrameAt, type MaskFrameScene } from "./maskFrame";
import { maskDraws } from "./maskPreview";

const W = 1920;
const H = 1080;
const T = 2000;

const panel = (x: number, y: number, w: number, h: number) => ({
  rect: [x, y, w, h] as [number, number, number, number],
  radius: 0.01,
  alpha: 1,
  ring_px: 0,
  ring_color: [0, 0, 0] as [number, number, number],
});

const preset = (sx: number): LayoutPresetDto => ({
  screen: panel(sx, 0.1, 0.7, 0.7),
  cam: panel(0.77, 0.574, 0.1875, 0.3333),
  arrangement: { screen: { cx: sx + 0.35, cy: 0.45, size: 0.7 }, cam: { cx: 0.86, cy: 0.74, size: 0.1875 } },
});

const PRESETS: LayoutPresets = {
  screen: preset(0.05),
  camera: preset(0.4),
  presenter: preset(0.2),
  screen_only: preset(0.6),
  camera_only: preset(0.8),
  segs: [],
  spans: [{ start_ms: 0, src: [0, 0, 1, 1], transition_ms: 350, fit: [1, 1] }],
  inset_w: 0.7,
};

const SEGS: LayoutSeg[] = [
  {
    id: "s0",
    start_ms: 0,
    end_ms: 8000,
    layout: "presenter",
    transition_ms: 0,
    easing: "smooth",
    transition_out_ms: 0,
    easing_out: "smooth",
  },
];

const FALLBACK: PreviewLayout = {
  screen: [0, 0, 1, 1],
  radius: 0,
  cam: null,
  canvas: [W, H],
  screenAlpha: 1,
  camAlpha: 0,
};

const ZOOM: ZoomSettings = {
  enabled: true,
  target_scale: 2.2,
  hold_ms: 900,
  smoothness: 0.5,
  clicks: 1,
  camera_shrink: true,
  camera_shrink_min: 0.6,
  smart_hold: true,
  smart_follow: true,
  camera_smoothing_ms: 220,
};

const SCENE: MaskFrameScene = {
  track: [
    { t: 0, scale: 1, cx: 0.5, cy: 0.5, curx: 0.5, cury: 0.5 },
    { t: 1000, scale: 2.2, cx: 0.32, cy: 0.61, curx: 0.32, cury: 0.61 },
    { t: 6000, scale: 2.2, cx: 0.32, cy: 0.61, curx: 0.32, cury: 0.61 },
  ],
  layout: FALLBACK,
  layoutPresets: PRESETS,
  layoutSegs: SEGS,
  cameraMoves: [],
  zooms: [],
  zoomSettings: ZOOM,
};

const MASK: EffectRegion = {
  id: "m0",
  kind: "blur",
  start_ms: 0,
  end_ms: 8000,
  fade_in_ms: 250,
  fade_out_ms: 250,
  layer: 0,
  rect: [0.35, 0.4, 0.3, 0.2],
};

function painterFrame() {
  const cam = camAt(SCENE.track, T);
  const base = layoutAt(SCENE.layoutSegs, SCENE.layoutPresets, T, [W, H]) ?? SCENE.layout;
  const layout = frameCamLayout(
    base,
    T,
    cam.scale,
    SCENE.cameraMoves,
    activeCamDraft(null, false),
    SCENE.zooms,
    SCENE.zoomSettings,
    W,
    H,
  );
  return { layout, cam };
}

const boxOf = (f: { layout: PreviewLayout | null; cam: { cx: number; cy: number; scale: number } }) =>
  maskDraws([MASK], f.layout, f.cam, W, H, T, 0.6)[0];

describe("maskFrameAt", () => {
  it("resolves the layout and the camera drawCompositeFrame resolves for the same instant", () => {
    const painter = painterFrame();
    expect(painter.cam.scale).toBeCloseTo(2.2, 6);
    expect(painter.layout?.screen).not.toEqual(FALLBACK.screen);
    expect(maskFrameAt(SCENE, T, W, H, null)).toEqual(painter);
  });
});

describe("the mask overlay box", () => {
  it("lands on the painted rectangle, not on the identity-camera one", () => {
    const painter = painterFrame();
    const live = boxOf(maskFrameAt(SCENE, T, W, H, null));
    expect(live).toEqual(boxOf(painter));
    const span = (m: typeof live) => m.mx[0] - m.mn[0];
    const flat = boxOf({ layout: painter.layout, cam: { cx: 0.5, cy: 0.5, scale: 1 } });
    expect(span(live) / span(flat)).toBeCloseTo(2.2, 2);
    const stale = boxOf({ layout: SCENE.layout, cam: { cx: 0.5, cy: 0.5, scale: 1 } });
    expect(span(live) / span(stale)).toBeCloseTo(2.2 * 0.7, 2);
    expect(Math.abs(live.mn[0] - stale.mn[0])).toBeGreaterThan(100);
  });
});
