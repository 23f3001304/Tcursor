import { describe, expect, it } from "vitest";
import { planProxySrc } from "./editorData";

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
