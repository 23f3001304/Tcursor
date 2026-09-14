import { describe, it, expect } from "vitest";
import { pickAddedCameraMoveId } from "./useTimelineActions";
import type { CameraMove } from "../../lib/edit";

const kf = (id: string, t_ms: number): CameraMove => ({ id, t_ms, x: 0.5, y: 0.5, size: 0.25, easing: "smooth", shape: "layout", roundness: 0.12 });

// M2: `AddCameraMove` sorts `camera_moves` by `t_ms` server-side, so the just-added keyframe's
// INDEX in the returned doc is not reliably `length - 1` the moment it lands anywhere but the
// end. `pickAddedCameraMoveId` must find it by diffing ids instead.
describe("pickAddedCameraMoveId", () => {
  it("finds the one id present in `after` but not `before`", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    const after = [kf("k0", 5000), kf("k1", 8000), kf("k2", 2000)]; // unsorted input is fine
    expect(pickAddedCameraMoveId(before, after)).toBe("k2");
  });

  it("finds it even after the server re-sorts by t_ms, landing it in the MIDDLE, not last", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    // The new keyframe (k2, t=2000) sorts to the FRONT - `after[length-1]` would wrongly pick k1.
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
