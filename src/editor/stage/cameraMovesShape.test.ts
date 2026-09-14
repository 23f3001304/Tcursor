import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../lib/edit";
import { camMoveAt, liveCamPose, overrideCamPanel, shapeRound } from "./cameraMoves";

// Mirrors moves_tests.rs `shapes_morph_between_keyframes` / `a_layout_shape_inherits_from_the_live_pose`
// and cam_tests.rs `override_camera_takes_the_poses_own_shape_when_it_has_one` - the Rust
// authority's shape cases, re-asserted in TS so the two never silently diverge. Split from
// `cameraMoves.test.ts` for its line budget.
const shaped = (t_ms: number, shape: CameraMove["shape"], roundness = 0): CameraMove =>
  ({ id: `k${t_ms}`, t_ms, x: 0.5, y: 0.5, size: 0.3, easing: "linear", shape, roundness });

describe("keyframe shapes", () => {
  it("morphs the corner fraction between keyframes like the rect", () => {
    const moves = [shaped(0, "circle"), shaped(1000, "rect")];
    expect(camMoveAt(moves, 0)?.round).toBe(0.5);
    expect(camMoveAt(moves, 1000)?.round).toBe(0);
    expect(camMoveAt(moves, 500)?.round).toBeCloseTo(0.25, 6);
    expect(camMoveAt([shaped(0, "rounded", 0.2)], 0)?.round).toBe(0.2);
    expect(shapeRound("rounded", 9)).toBe(0.5);
  });

  it("a layout shape inherits the live pose's, and stays unknown without one", () => {
    const live = { x: 0.9, y: 0.1, size: 0.2, round: 0.1 };
    expect(camMoveAt([shaped(2000, "layout")], 2000, live)?.round).toBe(0.1);
    expect(camMoveAt([shaped(2000, "layout")], 2000)?.round).toBeUndefined();
    const moves = [shaped(2000, "layout"), shaped(4000, "circle")];
    expect(camMoveAt(moves, 3000, live)?.round).toBeCloseTo(0.3, 6);
    expect(camMoveAt(moves, 3000)?.round).toBe(0.5); // one side unknown: the known side is carried
  });

  it("overrideCamPanel sets the radius from the pose's shape, else scales the static one", () => {
    // A 100x100 px static panel (r = 12 px) on a 1000x1000 canvas, as fractions.
    const base: [number, number, number, number, number, number, number, number, number] = [0, 0, 0.1, 0.1, 0.012, 0.004, 1, 2, 3];
    expect(overrideCamPanel(base, { x: 0.5, y: 0.5, size: 0.4, round: 0.5 }, 1000, 1000)[4]).toBeCloseTo(0.2, 6);
    expect(overrideCamPanel(base, { x: 0.5, y: 0.5, size: 0.4, round: 0 }, 1000, 1000)[4]).toBe(0);
    expect(overrideCamPanel(base, { x: 0.5, y: 0.5, size: 0.4 }, 1000, 1000)[4]).toBeCloseTo(0.048, 6);
    expect(liveCamPose(base, 1000, 1000).round).toBeCloseTo(0.12, 6);
  });
});
