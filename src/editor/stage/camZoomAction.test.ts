import { describe, it, expect } from "vitest";
import { applyCamZoomAction, camZoomAlpha, resolveCamAction, resolvedCamDefault, zoomProgress, type CamTuple } from "./camZoomAction";
import type { Zoom } from "../../lib/edit";
import type { ZoomSettings } from "../../hud/settings/settings";

const CAM: CamTuple = [0.1, 0.2, 0.2, 0.3, 0.05, 0.01, 255, 0, 0];
const TS = 2.2; // target_scale

// Hand-computed from the Rust: z = (scale-1)/(ts-1), smoothstep(z) = z*z*(3-2z),
// m = 1 + (to-1)*smoothstep(z). At scale 1.6 -> z = 0.5 -> smoothstep = 0.5 -> m = 0.81.
const zoom = (over: Partial<Zoom> = {}): Zoom => ({
  id: "z0", start_ms: 0, end_ms: 1000, target: "cursor", scale: 2.0, easing: "smooth",
  zoom_in_ms: 350, zoom_out_ms: 450, layer: 0, ...over,
});
const settings = (over: Partial<ZoomSettings> = {}): ZoomSettings => ({
  enabled: true, target_scale: TS, hold_ms: 2200, smoothness: 0.1, clicks: 1,
  camera_shrink: true, camera_shrink_min: 0.62, smart_hold: true, smart_follow: false, camera_smoothing_ms: 0, ...over,
});

describe("zoomProgress", () => {
  it("is 0 at rest, 1 at full zoom, and clamps beyond", () => {
    expect(zoomProgress(1.0, TS)).toBe(0);
    expect(zoomProgress(TS, TS)).toBe(1);
    expect(zoomProgress(0.5, TS)).toBe(0);
    expect(zoomProgress(9, TS)).toBe(1);
    expect(zoomProgress(1.6, TS)).toBeCloseTo(0.5, 10);
  });
});

describe("applyCamZoomAction (mirrors Rust apply_cam_zoom_action)", () => {
  // At m = 1 the result is the input, but it still round-trips through `cx - w/2`, so it lands
  // ~1e-16 off. The Rust does the identical recompute in `shrink_camera`, so this is parity,
  // not drift - assert closeness rather than bit-equality.
  it("is the identity at no zoom", () => {
    const out = applyCamZoomAction(CAM, { shrink: { to: 0.62 } }, 1.0, TS);
    out.forEach((v, i) => expect(v).toBeCloseTo(CAM[i], 12));
  });

  it("scales to `to` at full zoom, about the panel centre", () => {
    const out = applyCamZoomAction(CAM, { shrink: { to: 0.62 } }, TS, TS);
    expect(out[2]).toBeCloseTo(0.2 * 0.62, 10);
    expect(out[3]).toBeCloseTo(0.3 * 0.62, 10);
    // centre preserved
    expect(out[0] + out[2] / 2).toBeCloseTo(CAM[0] + CAM[2] / 2, 10);
    expect(out[1] + out[3] / 2).toBeCloseTo(CAM[1] + CAM[3] / 2, 10);
    // radius + ring ride the same multiplier; ring colour untouched
    expect(out[4]).toBeCloseTo(0.05 * 0.62, 10);
    expect(out[5]).toBeCloseTo(0.01 * 0.62, 10);
    expect([out[6], out[7], out[8]]).toEqual([255, 0, 0]);
  });

  it("uses the smoothstepped multiplier mid-zoom (m = 0.81 at z = 0.5)", () => {
    const out = applyCamZoomAction(CAM, { shrink: { to: 0.62 } }, 1.6, TS);
    expect(out[2]).toBeCloseTo(0.2 * 0.81, 10);
    expect(out[3]).toBeCloseTo(0.3 * 0.81, 10);
  });

  it("clamps `to` into 0.1..1 exactly like the Rust", () => {
    const tiny = applyCamZoomAction(CAM, { shrink: { to: 0 } }, TS, TS);
    expect(tiny[2]).toBeCloseTo(0.2 * 0.1, 10);
  });

  it("leaves geometry untouched for stay and hide", () => {
    expect(applyCamZoomAction(CAM, "stay", TS, TS)).toEqual(CAM);
    expect(applyCamZoomAction(CAM, "hide", TS, TS)).toEqual(CAM);
  });
});

describe("camZoomAlpha", () => {
  it("fades only for hide, on the same curve", () => {
    expect(camZoomAlpha("hide", 1.0, TS)).toBe(1);
    expect(camZoomAlpha("hide", TS, TS)).toBe(0);
    expect(camZoomAlpha("hide", 1.6, TS)).toBeCloseTo(0.5, 10);
    expect(camZoomAlpha("stay", TS, TS)).toBe(1);
    expect(camZoomAlpha({ shrink: { to: 0.62 } }, TS, TS)).toBe(1);
  });
});

describe("resolvedCamDefault (mirrors ZoomSettings::resolved_cam_action)", () => {
  it("derives from the legacy shrink fields when unset", () => {
    expect(resolvedCamDefault(settings())).toEqual({ shrink: { to: 0.62 } });
    expect(resolvedCamDefault(settings({ camera_shrink: false }))).toBe("stay");
  });
  it("prefers an explicit default", () => {
    expect(resolvedCamDefault(settings({ cam_zoom_default: "hide" }))).toBe("hide");
  });
});

describe("resolveCamAction (mirrors cam_action_at)", () => {
  const fallback = { shrink: { to: 0.62 } } as const;

  it("falls back to the global action AND scale with no zooms, or outside every zoom", () => {
    expect(resolveCamAction([], 500, fallback, TS)).toEqual([fallback, TS]);
    expect(resolveCamAction([zoom({ cam_action: "stay" })], 5000, fallback, TS)).toEqual([fallback, TS]);
  });

  it("an active zoom with no cam_action inherits the global ACTION but still reports its OWN scale", () => {
    // zoom()'s scale defaults to 2.0, distinct from the global fallback TS=2.2 - the winner's own
    // scale is never a "fallback" concern the way the action is, since Zoom.scale (unlike
    // cam_action) is never optional.
    expect(resolveCamAction([zoom()], 500, fallback, TS)).toEqual([fallback, 2.0]);
  });

  it("uses a per-zoom override inside the zoom", () => {
    expect(resolveCamAction([zoom({ cam_action: "stay" })], 500, fallback, TS)[0]).toBe("stay");
  });

  it("lets the highest layer win an overlap, like CameraSim", () => {
    const zs = [
      zoom({ id: "a", start_ms: 0, end_ms: 2000, layer: 0, cam_action: "stay" }),
      zoom({ id: "b", start_ms: 1000, end_ms: 3000, layer: 1, cam_action: "hide" }),
    ];
    expect(resolveCamAction(zs, 1500, fallback, TS)[0]).toBe("hide");
    expect(resolveCamAction(zs, 500, fallback, TS)[0]).toBe("stay");
  });

  // THE bug this task fixes: a zoom's own `scale` (a first-class per-zoom slider, presets
  // 1.6/2.2/2.8) must drive its shrink/hide progress, not the global `zoom.target_scale` -
  // otherwise a 1.6x zoom only ever reaches ~50% progress against a 2.2x global default.
  it("returns the winning zoom's own scale, not the global fallback scale", () => {
    const zs = [zoom({ cam_action: "hide", scale: 1.6 })];
    const [action, scale] = resolveCamAction(zs, 500, fallback, TS);
    expect(action).toBe("hide");
    expect(scale).toBe(1.6);
  });

  it("reaches full effect at the zoom's own peak scale, not the global one", () => {
    const zs = [zoom({ cam_action: "hide", scale: 1.6 })];
    const [action, scale] = resolveCamAction(zs, 500, fallback, TS);
    // live scale === the zoom's own peak (1.6, not TS=2.2) -> full hide.
    expect(camZoomAlpha(action, 1.6, scale)).toBe(0);
  });
});
