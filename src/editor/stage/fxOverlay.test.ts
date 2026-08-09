import { describe, it, expect, vi, beforeEach } from "vitest";
import type { FxOverlayParams } from "../../lib/ipc";

const sent: FxOverlayParams[] = [];
vi.mock("../../lib/ipc", () => ({
  previewFxOverlay: (p: FxOverlayParams) => { sent.push(p); return Promise.resolve("data:image/png;base64,x"); },
}));

const { requestFxOverlay } = await import("./fxOverlay");
const { newSpotlightSimState } = await import("./spotlightPreview");

const clickfx = {
  enabled: true, style: "ripple", color: [255, 0, 0], intensity: 1,
  spotlight: false, spotlight_dim: 0.6, spotlight_radius: 0.13, spotlight_feather: 0.1,
  spotlight_mode: "classic", spotlight_tint: [130, 90, 255], spotlight_dim_camera: true,
// eslint-disable-next-line @typescript-eslint/no-explicit-any
} as any;

const identity = (x: number, y: number): [number, number] => [x, y];
const call = (fx: unknown, clicks: { t: number; x: number; y: number }[], now: number) =>
  requestFxOverlay(100, 100, clicks as never, now, [50, 50],
    // spotlight always ON so the "enabled" gate is the only thing that can suppress it
    { effects: [], on: true, params: { dim: 0.6, radius: 0.13, feather: 0.1, mode: "classic", tint: [130, 90, 255] } },
    fx as never, identity, newSpotlightSimState(), 1, null);

describe("requestFxOverlay", () => {
  beforeEach(() => { sent.length = 0; });

  it("uses the export's 600ms click lifetime, not 500ms", async () => {
    // A click 550ms old is EXPIRED at 500ms but still alive (progress ~0.92) at the export's 600.
    await call(clickfx, [{ t: 0, x: 0.5, y: 0.5 }], 550);
    expect(sent[0].hits.length).toBe(1);
    expect(sent[0].hits[0][2]).toBeCloseTo(550 / 600, 5);
  });

  it("drops a click once it passes 600ms", async () => {
    await call(clickfx, [{ t: 0, x: 0.5, y: 0.5 }], 650);
    expect(sent[0].hits.length).toBe(0);
  });

  it("renders nothing at all when fx are disabled, spotlight included", async () => {
    // The export's `render` returns before drawing anything when `fx.enabled` is false.
    const out = await call({ ...clickfx, enabled: false }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(out).toBeNull();
    expect(sent.length).toBe(0);
  });

  it("still draws the spotlight when the click style is none", async () => {
    await call({ ...clickfx, style: "none" }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(sent[0].hits.length).toBe(0);
    expect(sent[0].spotAlpha).toBe(1);
  });
});
