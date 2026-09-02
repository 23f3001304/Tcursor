import { describe, expect, it } from "vitest";
import { hasWebcamSignal, planProxySrc } from "./editorData";
import type { PreviewLayout } from "../../lib/ipc";

describe("planProxySrc", () => {
  it("shows the raw capture immediately for a not-yet-preprocessed project with no proxy yet", () => {
    const plan = planProxySrc(480, 480, false, false);
    expect(plan.immediate).toBe("video.mp4");
    expect(plan.known).toBeNull();
    expect(plan.fetch).toBe(true);
  });

  it("points straight at the known proxy for a preprocessed project at the default quality", () => {
    const plan = planProxySrc(480, 480, true, false);
    expect(plan.immediate).toBeNull(); // preprocessed - never show raw 4K
    expect(plan.known).toBe("preview_480_rt.mp4");
    expect(plan.fetch).toBe(false);
  });

  it("still fetches for a preprocessed project at a non-default quality", () => {
    const plan = planProxySrc(720, 480, true, false);
    expect(plan.immediate).toBeNull();
    expect(plan.known).toBeNull();
    expect(plan.fetch).toBe(true);
  });

  it("does not re-show raw video once a proxy has already loaded for this folder", () => {
    const plan = planProxySrc(480, 480, false, true);
    expect(plan.immediate).toBeNull();
    expect(plan.fetch).toBe(true);
  });
});

describe("hasWebcamSignal (gate finding - camera lane hint shows for webcam-less recordings)", () => {
  const layoutWith = (cam: PreviewLayout["cam"]): PreviewLayout =>
    ({ screen: [0, 0, 1, 1], radius: 0, cam, canvas: [1920, 1080] });

  it("is false before the layout has resolved at all", () => {
    expect(hasWebcamSignal(null)).toBe(false);
  });

  it("is false when the starting layout has no camera panel (no webcam ever recorded)", () => {
    expect(hasWebcamSignal(layoutWith(null))).toBe(false);
  });

  it("is true when the starting layout has a camera panel", () => {
    expect(hasWebcamSignal(layoutWith([0.7, 0.7, 0.25, 0.25, 0.02, 0, 255, 255, 255]))).toBe(true);
  });
});
