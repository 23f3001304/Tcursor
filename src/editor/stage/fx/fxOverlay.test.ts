import { describe, it, expect, vi, beforeEach } from "vitest";
import type { FxOverlayParams } from "../../../shared/ipc";

const sent: FxOverlayParams[] = [];
let nextResult: () => Promise<string> = () => Promise.resolve("data:image/png;base64,x");
vi.mock("../../../shared/ipc", () => ({
  previewFxOverlay: (p: FxOverlayParams) => {
    sent.push(p);
    return nextResult();
  },
}));

const { requestFxOverlay } = await import("./fxOverlay");
const { resolveSpotlight, newSpotlightSimState } = await import("./spotlightPreview");

const clickfx = {
  enabled: true,
  style: "glow",
  color: [255, 0, 0],
  intensity: 1,
  spotlight: false,
  spotlight_dim: 0.6,
  spotlight_radius: 0.13,
  spotlight_feather: 0.1,
  spotlight_mode: "classic",
  spotlight_tint: [130, 90, 255],
  spotlight_dim_camera: true,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
} as any;

const identity = (x: number, y: number): [number, number] => [x, y];

const resolvedSpotlightOn = () =>
  resolveSpotlight(
    {
      effects: [],
      on: true,
      params: { dim: 0.6, radius: 0.13, feather: 0.1, mode: "classic", tint: [130, 90, 255] },
    },
    0,
    newSpotlightSimState(),
  )!;
const call = (fx: unknown, clicks: { t: number; x: number; y: number }[], now: number) =>
  requestFxOverlay(
    100,
    100,
    clicks as never,
    now,
    [50, 50],
    resolvedSpotlightOn(),
    fx as never,
    identity,
    1,
    null,
  );

describe("requestFxOverlay (sweep-2: clicks fall back to the overlay for an unmirrored style)", () => {
  beforeEach(() => {
    sent.length = 0;
    nextResult = () => Promise.resolve("data:image/png;base64,x");
  });

  it("includes click hits for an UNMIRRORED style (glow) - falls back to the overlay exactly as before", async () => {
    await call(clickfx, [{ t: 0, x: 0.5, y: 0.5 }], 550);
    expect(sent[0].hits.length).toBe(1);
    expect(sent[0].hits[0][2]).toBeCloseTo(550 / 600, 5);
  });

  it("drops an unmirrored-style click once it passes 600ms", async () => {
    await call(clickfx, [{ t: 0, x: 0.5, y: 0.5 }], 650);
    expect(sent[0].hits.length).toBe(0);
  });

  it("excludes click hits for a MIRRORED style (ripple) - ripplePreview.ts draws those client-side", async () => {
    await call({ ...clickfx, style: "ripple" }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(sent[0].hits).toEqual([]);
  });

  it("excludes click hits for a MIRRORED style (shockwave) too", async () => {
    await call({ ...clickfx, style: "shockwave" }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(sent[0].hits).toEqual([]);
  });

  it("excludes click hits for Pulse, mirrored client-side since the click-fx look pass", async () => {
    await call({ ...clickfx, style: "pulse" }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(sent[0].hits).toEqual([]);
  });

  it("excludes click hits when the style is none", async () => {
    await call({ ...clickfx, style: "none" }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(sent[0].hits).toEqual([]);
    expect(sent[0].spotAlpha).toBe(1);
  });

  it("triggers a backend call from an unmirrored-style click ALONE, spotlight off (unlike a mirrored style)", async () => {
    const out = await requestFxOverlay(
      100,
      100,
      [{ t: 0, x: 0.5, y: 0.5 }] as never,
      100,
      null,
      null,
      clickfx,
      identity,
      1,
      null,
    );
    expect(out).not.toBeNull();
    expect(sent.length).toBe(1);
    expect(sent[0].hits.length).toBe(1);
  });

  it("returns null with no backend call for a mirrored-style click ALONE, spotlight off", async () => {
    const out = await requestFxOverlay(
      100,
      100,
      [{ t: 0, x: 0.5, y: 0.5 }] as never,
      100,
      null,
      null,
      { ...clickfx, style: "ripple" },
      identity,
      1,
      null,
    );
    expect(out).toBeNull();
    expect(sent.length).toBe(0);
  });

  it("returns null with no backend call when nothing is active at all", async () => {
    const out = await requestFxOverlay(100, 100, [] as never, 100, null, null, clickfx, identity, 1, null);
    expect(out).toBeNull();
    expect(sent.length).toBe(0);
  });

  it("renders nothing at all when fx are disabled, spotlight included", async () => {
    const out = await call({ ...clickfx, enabled: false }, [{ t: 0, x: 0.5, y: 0.5 }], 100);
    expect(out).toBeNull();
    expect(sent.length).toBe(0);
  });

  it("rejects (does not resolve null) when the backend call fails", async () => {
    nextResult = () => Promise.reject(new Error("ipc down"));
    await expect(call(clickfx, [{ t: 0, x: 0.5, y: 0.5 }], 100)).rejects.toThrow("ipc down");
  });
});
