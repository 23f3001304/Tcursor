import { describe, it, expect } from "vitest";
import { pickAddedCameraMoveId } from "./useTimelineActions";
import type { CameraMove } from "../../../shared/edit";

const kf = (id: string, t_ms: number): CameraMove => ({
  id,
  t_ms,
  x: 0.5,
  y: 0.5,
  size: 0.25,
  easing: "smooth",
  shape: "layout",
  roundness: 0.12,
});

describe("pickAddedCameraMoveId", () => {
  it("finds the one id present in `after` but not `before`", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    const after = [kf("k0", 5000), kf("k1", 8000), kf("k2", 2000)];
    expect(pickAddedCameraMoveId(before, after)).toBe("k2");
  });

  it("finds it even after the server re-sorts by t_ms, landing it in the MIDDLE, not last", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    const after = [kf("k2", 2000), kf("k0", 5000), kf("k1", 8000)];
    expect(pickAddedCameraMoveId(before, after)).toBe("k2");
  });

  it("returns null when nothing changed (e.g. a failed apply left `after` === `before`)", () => {
    const before = [kf("k0", 5000)];
    expect(pickAddedCameraMoveId(before, before)).toBeNull();
  });

  it("returns null when both are empty", () => {
    expect(pickAddedCameraMoveId([], [])).toBeNull();
  });

  it("handles the first keyframe ever added (empty before)", () => {
    expect(pickAddedCameraMoveId([], [kf("k0", 1000)])).toBe("k0");
  });
});
