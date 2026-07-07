import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";
import { camMoveAt, radiusScaleForResize, rectFromCenter } from "./cameraMoves";

// Mirrors src-tauri/src/export/camera/moves_tests.rs - the Rust authority's cases,
// re-asserted in TS so the two never silently diverge.
const kf = (t_ms: number, x: number, y: number, size: number, easing: string): CameraMove =>
  ({ id: "k", t_ms, x, y, size, easing });

describe("camMoveAt", () => {
  it("empty track samples to null", () => {
    expect(camMoveAt([], 0)).toBeNull();
    expect(camMoveAt([], 5000)).toBeNull();
  });

  it("single keyframe holds at any time", () => {
    const moves = [kf(1000, 0.3, 0.7, 0.2, "smooth")];
    const want = { x: 0.3, y: 0.7, size: 0.2 };
    expect(camMoveAt(moves, 0)).toEqual(want);
    expect(camMoveAt(moves, 1000)).toEqual(want);
    expect(camMoveAt(moves, 50_000)).toEqual(want);
  });

  it("holds first pose before and at first keyframe", () => {
    const moves = [kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    const first = { x: 0.1, y: 0.1, size: 0.1 };
    expect(camMoveAt(moves, 0)).toEqual(first);
    expect(camMoveAt(moves, 999)).toEqual(first);
    expect(camMoveAt(moves, 1000)).toEqual(first);
  });

  it("holds last pose at and after last keyframe", () => {
    const moves = [kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    const last = { x: 0.9, y: 0.9, size: 0.5 };
    expect(camMoveAt(moves, 2000)).toEqual(last);
    expect(camMoveAt(moves, 9000)).toEqual(last);
  });

  it("midpoint lerp with linear easing is the exact mean", () => {
    const moves = [kf(0, 0.0, 0.0, 0.0, "linear"), kf(1000, 1.0, 1.0, 1.0, "linear")];
    const p = camMoveAt(moves, 500)!;
    expect(p.x).toBeCloseTo(0.5, 6);
    expect(p.y).toBeCloseTo(0.5, 6);
    expect(p.size).toBeCloseTo(0.5, 6);
  });

  it("midpoint with smooth easing diverges from the linear mean", () => {
    // smoothstep(0.5) == 0.5 exactly, so a symmetric pair alone wouldn't distinguish
    // smooth from linear - use asymmetric per-axis deltas, assert against ease() directly.
    const moves = [kf(0, 0.0, 0.2, 0.1, "smooth"), kf(1000, 1.0, 0.2, 0.9, "smooth")];
    const f = ease("smooth", 0.25); // != 0.25 for smoothstep away from the midpoint
    expect(Math.abs(f - 0.25)).toBeGreaterThan(1e-6);
    const p = camMoveAt(moves, 250)!;
    expect(p.x).toBeCloseTo(f, 6);
    expect(p.size).toBeCloseTo(0.1 + 0.8 * f, 6);
  });

  it("coincident keyframe times snap to b without dividing by zero", () => {
    const moves = [kf(1000, 0.2, 0.2, 0.2, "linear"), kf(1000, 0.8, 0.8, 0.8, "linear"), kf(2000, 0.0, 0.0, 0.0, "linear")];
    const p = camMoveAt(moves, 1000)!;
    expect(p.x === 0.2 || p.x === 0.8).toBe(true);
  });

  it("out-of-order input is sorted defensively, and the input array is not mutated", () => {
    const moves = [kf(2000, 0.9, 0.9, 0.5, "linear"), kf(0, 0.0, 0.0, 0.0, "linear")];
    const snapshot = [...moves];
    expect(camMoveAt(moves, 0)).toEqual({ x: 0.0, y: 0.0, size: 0.0 });
    expect(camMoveAt(moves, 2000)).toEqual({ x: 0.9, y: 0.9, size: 0.5 });
    const mid = camMoveAt(moves, 1000)!;
    expect(mid.x).toBeCloseTo(0.45, 6);
    expect(moves).toEqual(snapshot); // camMoveAt must sort a copy, not the caller's array
  });

  // Mirrors moves_tests.rs's single_keyframe_with_static_pose_animates_in_from_the_static_start -
  // same numbers, so the two never silently diverge on the "implicit start keyframe" behavior.
  it("single keyframe with a static pose animates in from the static start", () => {
    const moves = [kf(1000, 0.9, 0.9, 0.5, "linear")];
    const staticPose = { x: 0.1, y: 0.1, size: 0.1 };

    const at0 = camMoveAt(moves, 0, staticPose)!;
    expect(at0.x).toBeCloseTo(staticPose.x, 6);
    expect(at0.y).toBeCloseTo(staticPose.y, 6);
    expect(at0.size).toBeCloseTo(staticPose.size, 6);

    const want = { x: 0.9, y: 0.9, size: 0.5 };
    const at1000 = camMoveAt(moves, 1000, staticPose)!;
    expect(at1000.x).toBeCloseTo(want.x, 6);
    expect(at1000.y).toBeCloseTo(want.y, 6);
    expect(at1000.size).toBeCloseTo(want.size, 6);

    const mid = camMoveAt(moves, 500, staticPose)!;
    expect(mid.x).toBeGreaterThan(staticPose.x);
    expect(mid.x).toBeLessThan(want.x);
    expect(mid.y).toBeGreaterThan(staticPose.y);
    expect(mid.y).toBeLessThan(want.y);
    expect(mid.size).toBeGreaterThan(staticPose.size);
    expect(mid.size).toBeLessThan(want.size);

    // Omitting staticPose still means the old "hold the keyframe flat" behavior, at the same t=500.
    const held = camMoveAt(moves, 500)!;
    expect(held).toEqual(want);
  });
});

// Task 9 Part C: mirrors export::scene::mod_tests::override_camera_* - the preview must scale
// the spliced radius fraction by the same height ratio the Rust `override_camera` uses, so a
// circle stays a true circle after a keyframe resizes the panel (not just in the export).
describe("radiusScaleForResize", () => {
  it("shrinking the panel shrinks the scale proportionally (half height -> half radius)", () => {
    expect(radiusScaleForResize(0.2, 0.1)).toBeCloseTo(0.5, 6);
  });

  it("growing the panel grows the scale proportionally (4x height -> 4x radius)", () => {
    expect(radiusScaleForResize(0.1, 0.4)).toBeCloseTo(4.0, 6);
  });

  it("no size change is a no-op scale of 1", () => {
    expect(radiusScaleForResize(0.25, 0.25)).toBe(1);
  });

  it("guards a zero/near-zero old height against dividing by zero (matches Rust's old_h.max(0.001))", () => {
    expect(Number.isFinite(radiusScaleForResize(0, 0.1))).toBe(true);
    expect(radiusScaleForResize(0, 0.1)).toBeCloseTo(0.1 / 0.001, 6);
  });

  it("end-to-end: a circle's radius stays min(w,h)/2 IN PIXELS after resize, mirroring the export", () => {
    // PreviewLayout.cam[4] is a WIDTH-relative fraction (radius_px / ow - see previewCanvas.ts's
    // `wr = fr * w`), while cam[2]/cam[3] (w/h) are relative to ow/oh respectively - so comparing
    // fractions directly (as in Rust, where both are already px) is invalid whenever ow != oh.
    // Converting each back to pixels with its own axis is what actually proves parity.
    const ow = 1280, oh = 720;
    const oldHFrac = 0.2, oldHPx = oldHFrac * oh; // static panel: square in px, height 0.2 of oh
    const staticRadiusFrac = (oldHPx / 2) / ow;    // radius_px (== oldHPx/2 for a circle) / ow
    const cp = { x: 0.5, y: 0.5, size: 0.05 };     // shrinks to a 0.05-tall (of oh) panel
    const [, , , newHFrac] = rectFromCenter(cp, ow, oh);
    const newHPx = newHFrac * oh;
    const scaledRadiusFrac = staticRadiusFrac * radiusScaleForResize(oldHFrac, newHFrac);
    const scaledRadiusPx = scaledRadiusFrac * ow;
    expect(scaledRadiusPx).toBeCloseTo(newHPx / 2, 5); // still a true circle: radius == h/2 in px
  });
});
